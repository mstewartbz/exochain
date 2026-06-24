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

import { createHash } from 'node:crypto';
import type { ContactRateLimitBucket } from './contact-intake-policy';

export type ContactSubmissionInput = {
  name: string;
  email: string;
  organization: string;
  role: string;
  intendedUse: string;
  userAgent: string;
  forwardedFor: string;
};

export type ContactSubmission = ContactSubmissionInput & {
  id: string;
  submittedAt: string;
  notificationStatus: string;
  notificationError: string;
};

type SiteDagDbConfig = {
  gatewayUrl: string;
  authToken: string;
  tenantId: string;
  namespace: string;
  ownerDid: string;
  controllerDid: string;
  submittedByDid: string;
  writeSignature: string;
};

type SiteContactResult = {
  submission?: unknown;
  submissions?: unknown;
  notification_updated?: unknown;
  request_count?: unknown;
};

const SITE_DAGDB_GATEWAY_URL_ENV = 'SITE_DAGDB_GATEWAY_URL';
const SITE_DAGDB_AUTH_TOKEN_ENV = 'SITE_DAGDB_AUTH_TOKEN';
const SITE_DAGDB_TENANT_ID_ENV = 'SITE_DAGDB_TENANT_ID';
const SITE_DAGDB_NAMESPACE_ENV = 'SITE_DAGDB_NAMESPACE';
const SITE_DAGDB_OWNER_DID_ENV = 'SITE_DAGDB_OWNER_DID';
const SITE_DAGDB_CONTROLLER_DID_ENV = 'SITE_DAGDB_CONTROLLER_DID';
const SITE_DAGDB_SUBMITTED_BY_DID_ENV = 'SITE_DAGDB_SUBMITTED_BY_DID';
const SITE_DAGDB_WRITE_SIGNATURE_ENV = 'SITE_DAGDB_WRITE_SIGNATURE';
const SITE_CONTACT_RESULT_FIELD = 'site_contact_result';
const CONTACT_INTAKE_PATH = '/api/v1/dag-db/intake';
const MAX_TEXT_LENGTH = 1000;

function normalizeSecret(value: string | undefined): string {
  let normalized = value?.trim() || '';
  const hasDoubleQuotes = normalized.startsWith('"') && normalized.endsWith('"');
  const hasSingleQuotes = normalized.startsWith("'") && normalized.endsWith("'");

  if (normalized.length >= 2 && (hasDoubleQuotes || hasSingleQuotes)) {
    normalized = normalized.slice(1, -1).trim();
  }

  return normalized;
}

function requireEnv(env: NodeJS.ProcessEnv, key: string): string {
  const value = normalizeSecret(env[key]);
  if (!value) {
    throw new Error(`Site contact DAG DB adapter is not configured. Set ${key}.`);
  }
  return value;
}

function requireSiteDagDbConfig(env: NodeJS.ProcessEnv = process.env): SiteDagDbConfig {
  const gatewayUrl = requireEnv(env, SITE_DAGDB_GATEWAY_URL_ENV).replace(/\/+$/u, '');
  return {
    gatewayUrl,
    authToken: requireEnv(env, SITE_DAGDB_AUTH_TOKEN_ENV),
    tenantId: requireEnv(env, SITE_DAGDB_TENANT_ID_ENV),
    namespace: requireEnv(env, SITE_DAGDB_NAMESPACE_ENV),
    ownerDid: requireEnv(env, SITE_DAGDB_OWNER_DID_ENV),
    controllerDid: requireEnv(env, SITE_DAGDB_CONTROLLER_DID_ENV),
    submittedByDid: requireEnv(env, SITE_DAGDB_SUBMITTED_BY_DID_ENV),
    writeSignature: requireEnv(env, SITE_DAGDB_WRITE_SIGNATURE_ENV),
  };
}

function truncate(value: string): string {
  return value.slice(0, MAX_TEXT_LENGTH);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stableJson(value: unknown): string {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableJson(item)).join(',')}]`;
  }

  const record = value as Record<string, unknown>;
  return `{${Object.keys(record)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${stableJson(record[key])}`)
    .join(',')}}`;
}

function sha256Hex(value: unknown): string {
  return createHash('sha256').update(stableJson(value)).digest('hex');
}

function contactResult(value: unknown): SiteContactResult {
  if (!isRecord(value) || !isRecord(value[SITE_CONTACT_RESULT_FIELD])) {
    throw new Error('Site contact DAG DB response missing site_contact_result; refusing to synthesize contact state.');
  }
  return value[SITE_CONTACT_RESULT_FIELD] as SiteContactResult;
}

function isContactSubmission(value: unknown): value is ContactSubmission {
  if (!isRecord(value)) {
    return false;
  }

  return [
    'id',
    'submittedAt',
    'name',
    'email',
    'organization',
    'role',
    'intendedUse',
    'userAgent',
    'forwardedFor',
    'notificationStatus',
    'notificationError',
  ].every((field) => typeof value[field] === 'string');
}

function contactSubmissionInput(input: ContactSubmissionInput): ContactSubmissionInput {
  return {
    name: truncate(input.name),
    email: truncate(input.email),
    organization: truncate(input.organization),
    role: truncate(input.role),
    intendedUse: truncate(input.intendedUse),
    userAgent: truncate(input.userAgent),
    forwardedFor: truncate(input.forwardedFor),
  };
}

async function readJson(response: Response): Promise<unknown> {
  const text = await response.text();
  if (!text) {
    return {};
  }

  try {
    return JSON.parse(text) as unknown;
  } catch (error) {
    throw new Error(`Site contact DAG DB response was not JSON: ${error instanceof Error ? error.message : String(error)}`);
  }
}

async function postContactIntake(
  kind: string,
  operation: unknown,
  consentPurpose: 'retrieval' | 'writeback',
  parentMemoryIds: string[] | null = null,
): Promise<SiteContactResult> {
  const config = requireSiteDagDbConfig();
  const sourceMaterial = {
    kind,
    operation,
    contract: 'site.contact.dagdb.v1',
  };
  const sourceHash = sha256Hex(sourceMaterial);
  const payloadHash = sha256Hex(operation);
  const idempotencyKey = `site-contact-${kind}-${sha256Hex({
    sourceHash,
    payloadHash,
    tenantId: config.tenantId,
    namespace: config.namespace,
  })}`;
  const body = {
    tenant_id: config.tenantId,
    namespace: config.namespace,
    idempotency_key: idempotencyKey,
    source_type: 'generated',
    source_hash: sourceHash,
    payload_hash: payloadHash,
    owner_did: config.ownerDid,
    controller_did: config.controllerDid,
    submitted_by_did: config.submittedByDid,
    consent_purpose: consentPurpose,
    requested_action: `site:contact:${kind}`,
    title_text: `site contact ${kind}`,
    summary_text: stableJson(operation).slice(0, MAX_TEXT_LENGTH),
    payload_uri_hash: null,
    parent_memory_ids: parentMemoryIds,
    edge_types: null,
    access_policy_hash: null,
    declared_rights_hash: null,
    keyword_texts: ['site', 'contact', kind],
  };

  const response = await fetch(`${config.gatewayUrl}${CONTACT_INTAKE_PATH}`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${config.authToken}`,
      'Content-Type': 'application/json',
      'x-exo-tenant-id': config.tenantId,
      'x-exo-namespace': config.namespace,
      'x-exo-authority-scope': `dagdb:intake:${config.tenantId}:${config.namespace}`,
      'x-exo-write-signature': config.writeSignature,
    },
    body: JSON.stringify(body),
  });

  const parsed = await readJson(response);
  if (!response.ok) {
    throw new Error(`Site contact DAG DB intake failed with status ${response.status}: ${stableJson(parsed).slice(0, MAX_TEXT_LENGTH)}`);
  }

  return contactResult(parsed);
}

export async function ensureContactSubmissionSchema(): Promise<void> {
  requireSiteDagDbConfig();
}

export async function createContactSubmission(
  input: ContactSubmissionInput,
): Promise<ContactSubmission> {
  const result = await postContactIntake(
    'submission',
    { submission: contactSubmissionInput(input) },
    'writeback',
  );

  if (!isContactSubmission(result.submission)) {
    throw new Error('Site contact DAG DB submission response is missing a valid submission record.');
  }

  return result.submission;
}

async function incrementContactRateLimit(
  bucket: ContactRateLimitBucket,
): Promise<number> {
  const result = await postContactIntake(
    'rate-limit',
    {
      bucket: {
        bucket: bucket.bucket,
        maxRequests: bucket.maxRequests,
        windowSeconds: bucket.windowSeconds,
      },
    },
    'writeback',
  );
  const requestCount = Number(result.request_count);

  if (!Number.isInteger(requestCount) || requestCount < 0) {
    throw new Error('Site contact DAG DB rate-limit response is missing request_count.');
  }

  return requestCount;
}

export async function assertContactSubmissionRateLimit(
  buckets: ContactRateLimitBucket[],
): Promise<boolean> {
  for (const bucket of buckets) {
    const requestCount = await incrementContactRateLimit(bucket);
    if (requestCount > bucket.maxRequests) {
      return false;
    }
  }

  return true;
}

export async function updateContactSubmissionNotification(
  id: string,
  status: 'sent' | 'not_configured' | 'failed',
  error: string,
): Promise<void> {
  const result = await postContactIntake(
    'notification',
    {
      id,
      status,
      error: truncate(error),
    },
    'writeback',
    [id],
  );

  if (result.notification_updated !== true) {
    throw new Error('Site contact DAG DB notification response did not confirm notification_updated.');
  }
}

export async function listRecentContactSubmissions(): Promise<ContactSubmission[]> {
  const result = await postContactIntake('recent', { limit: 50 }, 'retrieval');

  if (!Array.isArray(result.submissions) || !result.submissions.every(isContactSubmission)) {
    throw new Error('Site contact DAG DB recent response is missing submission records.');
  }

  return result.submissions;
}
