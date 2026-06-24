// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

export const DAGDB_DURABLE_STATE_FAMILIES = [
  'council-tickets',
  'council-conversations',
  'feedback-issues',
  'layout-templates',
  'ape-onboarding',
] as const

export type DagDbDurableStateFamily = typeof DAGDB_DURABLE_STATE_FAMILIES[number]

type DagDbDurableStateConfig = {
  gatewayUrl: string
  tenantId: string
  namespace: string
  ownerDid: string
  controllerDid: string
  submittedByDid: string
  token: string
}

type WebDurableStateResult = {
  value?: unknown
  stored?: unknown
  deleted?: unknown
}

const DAGDB_INTAKE_PATH = '/api/v1/dag-db/intake'
const WEB_DURABLE_STATE_RESULT_FIELD = 'web_durable_state_result'
const DURABLE_STATE_CACHE = new Map<DagDbDurableStateFamily, unknown>()

function envValue(name: keyof ImportMetaEnv, fallback: string): string {
  const value = import.meta.env[name]
  return typeof value === 'string' && value.trim().length > 0 ? value.trim() : fallback
}

function authToken(): string {
  try {
    return localStorage.getItem('df_token') || ''
  } catch {
    return ''
  }
}

function config(): DagDbDurableStateConfig {
  return {
    gatewayUrl: envValue('VITE_DAGDB_GATEWAY_URL', '').replace(/\/+$/u, ''),
    tenantId: envValue('VITE_DAGDB_TENANT_ID', 'web-local-dev'),
    namespace: envValue('VITE_DAGDB_NAMESPACE', 'decision-forum-web'),
    ownerDid: envValue('VITE_DAGDB_OWNER_DID', 'did:exo:web-user'),
    controllerDid: envValue('VITE_DAGDB_CONTROLLER_DID', 'did:exo:web-controller'),
    submittedByDid: envValue('VITE_DAGDB_SUBMITTED_BY_DID', 'did:exo:web-client'),
    token: authToken(),
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function cloneForCache<T>(value: T): T {
  if (typeof structuredClone === 'function') {
    return structuredClone(value)
  }
  return JSON.parse(JSON.stringify(value)) as T
}

function stableJson(value: unknown): string {
  if (value === null || typeof value === 'undefined') return 'null'
  if (Array.isArray(value)) return `[${value.map(stableJson).join(',')}]`
  if (typeof value === 'object') {
    const record = value as Record<string, unknown>
    return `{${Object.keys(record)
      .sort()
      .map(key => `${JSON.stringify(key)}:${stableJson(record[key])}`)
      .join(',')}}`
  }
  return JSON.stringify(value)
}

async function sha256Hex(value: unknown): Promise<string> {
  const bytes = new TextEncoder().encode(stableJson(value))
  const digest = await crypto.subtle.digest('SHA-256', bytes)
  return Array.from(new Uint8Array(digest))
    .map(byte => byte.toString(16).padStart(2, '0'))
    .join('')
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text()
  if (!text) return {}
  try {
    return JSON.parse(text) as unknown
  } catch (error) {
    throw new Error(`DAG DB durable state response was not JSON: ${error instanceof Error ? error.message : String(error)}`)
  }
}

function durableStateResult(value: unknown): WebDurableStateResult {
  if (!isRecord(value) || !isRecord(value[WEB_DURABLE_STATE_RESULT_FIELD])) {
    throw new Error('DAG DB durable state response missing web_durable_state_result')
  }
  return value[WEB_DURABLE_STATE_RESULT_FIELD] as WebDurableStateResult
}

async function postDagDbDurableState(
  family: DagDbDurableStateFamily,
  operation: 'read' | 'write' | 'delete',
  value: unknown,
): Promise<WebDurableStateResult> {
  const cfg = config()
  const payload = {
    adapter: 'web-dagdb-durable-state-v1',
    family,
    operation,
    value,
  }
  const payloadHash = await sha256Hex(payload)
  const sourceHash = await sha256Hex({
    adapter: 'web-dagdb-durable-state-v1',
    family,
    operation,
  })
  const body = {
    tenant_id: cfg.tenantId,
    namespace: cfg.namespace,
    idempotency_key: `web-durable-state:${family}:${operation}:${payloadHash.slice(0, 48)}`,
    source_type: 'generated',
    source_hash: sourceHash,
    payload_hash: payloadHash,
    owner_did: cfg.ownerDid,
    controller_did: cfg.controllerDid,
    submitted_by_did: cfg.submittedByDid,
    consent_purpose: operation === 'read' ? 'retrieval' : 'writeback',
    requested_action: `web:durable-state:${family}:${operation}`,
    title_text: `web durable state ${family}`,
    summary_text: `web durable state ${family} ${operation} ${payloadHash}`,
    payload_uri_hash: null,
    parent_memory_ids: null,
    edge_types: null,
    access_policy_hash: null,
    declared_rights_hash: null,
    keyword_texts: ['web', 'durable-state', family, operation],
  }

  const response = await fetch(`${cfg.gatewayUrl}${DAGDB_INTAKE_PATH}`, {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
      Authorization: cfg.token ? `Bearer ${cfg.token}` : '',
      'x-exo-tenant-id': cfg.tenantId,
      'x-exo-namespace': cfg.namespace,
      'x-exo-authority-scope': `dagdb:intake:${cfg.tenantId}:${cfg.namespace}`,
    },
    body: JSON.stringify(body),
  })
  const parsed = await readJson(response)
  if (!response.ok) {
    throw new Error(`DAG DB durable state ${family} ${operation} failed with status ${response.status}: ${stableJson(parsed).slice(0, 500)}`)
  }
  return durableStateResult(parsed)
}

export function cacheDagDbDurableState<T>(
  family: DagDbDurableStateFamily,
  value: T,
): void {
  DURABLE_STATE_CACHE.set(family, cloneForCache(value))
}

export function readCachedDagDbDurableState<T>(
  family: DagDbDurableStateFamily,
  fallback: T,
): T {
  if (!DURABLE_STATE_CACHE.has(family)) return fallback
  return cloneForCache(DURABLE_STATE_CACHE.get(family) as T)
}

export async function persistDagDbDurableState<T>(
  family: DagDbDurableStateFamily,
  value: T,
): Promise<void> {
  cacheDagDbDurableState(family, value)
  const result = await postDagDbDurableState(family, 'write', value)
  if (result.stored !== true) {
    throw new Error(`DAG DB durable state ${family} write did not confirm stored=true`)
  }
}

export async function hydrateDagDbDurableState<T>(
  family: DagDbDurableStateFamily,
  fallback: T,
): Promise<T> {
  const result = await postDagDbDurableState(family, 'read', null)
  const value = Object.prototype.hasOwnProperty.call(result, 'value') ? result.value : fallback
  cacheDagDbDurableState(family, value as T)
  return cloneForCache(value as T)
}

export async function deleteDagDbDurableState(
  family: DagDbDurableStateFamily,
): Promise<void> {
  DURABLE_STATE_CACHE.delete(family)
  const result = await postDagDbDurableState(family, 'delete', null)
  if (result.deleted !== true) {
    throw new Error(`DAG DB durable state ${family} delete did not confirm deleted=true`)
  }
}

export function resetDagDbDurableStateForTests(): void {
  DURABLE_STATE_CACHE.clear()
}
