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

import { readFileSync } from 'node:fs'
import path from 'node:path'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import {
  cacheDagDbDurableState,
  deleteDagDbDurableState,
  hydrateDagDbDurableState,
  persistDagDbDurableState,
  readCachedDagDbDurableState,
  resetDagDbDurableStateForTests,
} from './dagdbDurableState'

const webRoot = path.resolve(__dirname, '../..')

function source(relPath: string): string {
  return readFileSync(path.join(webRoot, relPath), 'utf8')
}

function mockDagDbFetch(body: string, ok = true, status = 200) {
  const fetchMock = vi.fn(async () => ({
    ok,
    status,
    text: async () => body,
  } as Response))
  vi.stubGlobal('fetch', fetchMock)
  return fetchMock
}

describe('DAG DB durable web state contract', () => {
  beforeEach(() => {
    resetDagDbDurableStateForTests()
    localStorage.clear()
    localStorage.setItem('df_token', 'web-test-token')
  })

  afterEach(() => {
    vi.unstubAllGlobals()
    resetDagDbDurableStateForTests()
    localStorage.clear()
  })

  it('keeps durable product state off browser localStorage keys', () => {
    const durableSource = [
      'src/lib/council.ts',
      'src/stores/feedbackStore.ts',
      'src/stores/layoutTemplateStore.ts',
      'src/pages/APE/OnboardPage.tsx',
      'src/pages/APE/APEDashboardPage.tsx',
      'src/lib/auth.tsx',
    ].map(source).join('\n')

    expect(durableSource).not.toMatch(/df_council_tickets|df_council_conversations/)
    expect(durableSource).not.toMatch(/exo_feedback_issues/)
    expect(durableSource).not.toMatch(/exo_layout_templates|exo_active_template_id/)
    expect(durableSource).not.toMatch(/ape_onboarding/)
  })

  it('uses the DAG DB intake adapter for durable product state families', () => {
    const adapter = source('src/lib/dagdbDurableState.ts')

    expect(adapter).toMatch(/\/api\/v1\/dag-db\/intake/)
    expect(adapter).toMatch(/web_durable_state_result/)
    expect(adapter).toMatch(/x-exo-authority-scope/)
    expect(adapter).toMatch(/council-tickets/)
    expect(adapter).toMatch(/council-conversations/)
    expect(adapter).toMatch(/feedback-issues/)
    expect(adapter).toMatch(/layout-templates/)
    expect(adapter).toMatch(/ape-onboarding/)
  })

  it('persists durable state through DAG DB intake and keeps a cloned cache', async () => {
    const fetchMock = mockDagDbFetch(JSON.stringify({
      web_durable_state_result: { stored: true },
    }))
    const value = [{ id: 'issue-1', status: 'open' }]

    await persistDagDbDurableState('feedback-issues', value)

    expect(fetchMock).toHaveBeenCalledTimes(1)
    const [url, init] = fetchMock.mock.calls[0]
    expect(url).toBe('/api/v1/dag-db/intake')
    expect((init as RequestInit).method).toBe('POST')
    expect(((init as RequestInit).headers as Record<string, string>).Authorization).toBe('Bearer web-test-token')
    expect(((init as RequestInit).headers as Record<string, string>)['x-exo-authority-scope'])
      .toBe('dagdb:intake:web-local-dev:decision-forum-web')
    const body = JSON.parse(String((init as RequestInit).body))
    expect(body.requested_action).toBe('web:durable-state:feedback-issues:write')
    expect(body.consent_purpose).toBe('writeback')
    expect(body.keyword_texts).toEqual(['web', 'durable-state', 'feedback-issues', 'write'])

    value[0].status = 'closed'
    expect(readCachedDagDbDurableState('feedback-issues', [])).toEqual([{ id: 'issue-1', status: 'open' }])
  })

  it('hydrates and deletes durable state through DAG DB intake', async () => {
    const fetchMock = mockDagDbFetch(JSON.stringify({
      web_durable_state_result: { value: { done: false } },
    }))

    await expect(hydrateDagDbDurableState('ape-onboarding', { done: true }))
      .resolves.toEqual({ done: false })
    expect(readCachedDagDbDurableState('ape-onboarding', null)).toEqual({ done: false })
    expect(JSON.parse(String((fetchMock.mock.calls[0][1] as RequestInit).body)).consent_purpose).toBe('retrieval')

    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      text: async () => JSON.stringify({ web_durable_state_result: { deleted: true } }),
    } as Response)
    await expect(deleteDagDbDurableState('ape-onboarding')).resolves.toBeUndefined()
    expect(readCachedDagDbDurableState('ape-onboarding', 'fallback')).toBe('fallback')
  })

  it('fails closed on missing confirmations and malformed gateway responses', async () => {
    mockDagDbFetch(JSON.stringify({ web_durable_state_result: { stored: false } }))
    await expect(persistDagDbDurableState('layout-templates', { id: 'layout-1' }))
      .rejects.toThrow(/stored=true/)

    mockDagDbFetch(JSON.stringify({ web_durable_state_result: { deleted: false } }))
    await expect(deleteDagDbDurableState('layout-templates')).rejects.toThrow(/deleted=true/)

    mockDagDbFetch(JSON.stringify({ wrong_result: {} }))
    await expect(hydrateDagDbDurableState('layout-templates', null))
      .rejects.toThrow(/missing web_durable_state_result/)

    mockDagDbFetch('not-json')
    await expect(hydrateDagDbDurableState('layout-templates', null))
      .rejects.toThrow(/not JSON/)

    mockDagDbFetch(JSON.stringify({ error: 'unavailable' }), false, 503)
    await expect(hydrateDagDbDurableState('layout-templates', null))
      .rejects.toThrow(/failed with status 503/)
  })

  it('uses cached fallback paths without mutating callers', () => {
    const fallback = { mode: 'fallback' }
    expect(readCachedDagDbDurableState('council-tickets', fallback)).toBe(fallback)

    const cached = { mode: 'cached' }
    cacheDagDbDurableState('council-tickets', cached)
    cached.mode = 'changed'
    expect(readCachedDagDbDurableState('council-tickets', fallback)).toEqual({ mode: 'cached' })
  })
})
