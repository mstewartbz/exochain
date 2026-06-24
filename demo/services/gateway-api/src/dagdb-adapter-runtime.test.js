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

import { afterEach, describe, expect, it, vi } from 'vitest';
import {
  createDemoServiceStore,
  getDemoServiceTestStore,
  requireDemoDagDbConfig,
  resetDemoServiceTestStore,
} from '../../../packages/shared/src/dagdb-adapter.js';

const COMPLETE_ENV = {
  EXO_DEMO_DAGDB_GATEWAY_URL: 'https://gateway.example.test///',
  EXO_DEMO_DAGDB_AUTH_TOKEN: 'demo-auth-token',
  EXO_DEMO_DAGDB_TENANT_ID: 'tenant-a',
  EXO_DEMO_DAGDB_NAMESPACE: 'demo-namespace',
  EXO_DEMO_DAGDB_OWNER_DID: 'did:exo:owner',
  EXO_DEMO_DAGDB_CONTROLLER_DID: 'did:exo:controller',
  EXO_DEMO_DAGDB_SUBMITTED_BY_DID: 'did:exo:submitter',
  EXO_DEMO_DAGDB_WRITE_SIGNATURE: 'signature-1',
};

const ORIGINAL_FETCH = globalThis.fetch;
const ORIGINAL_VITEST = process.env.VITEST;
const HAD_VITEST = Object.prototype.hasOwnProperty.call(process.env, 'VITEST');

function withProductionStore(fn) {
  delete process.env.VITEST;
  return Promise.resolve()
    .then(fn)
    .finally(() => {
      if (HAD_VITEST) {
        process.env.VITEST = ORIGINAL_VITEST;
      } else {
        delete process.env.VITEST;
      }
    });
}

function okResponse(body) {
  return {
    ok: true,
    status: 200,
    text: async () => JSON.stringify(body),
  };
}

function lastRequestBody() {
  const [, init] = globalThis.fetch.mock.calls.at(-1);
  return JSON.parse(init.body);
}

afterEach(() => {
  if (ORIGINAL_FETCH) {
    globalThis.fetch = ORIGINAL_FETCH;
  } else {
    delete globalThis.fetch;
  }
  if (HAD_VITEST) {
    process.env.VITEST = ORIGINAL_VITEST;
  } else {
    delete process.env.VITEST;
  }
  resetDemoServiceTestStore();
  vi.restoreAllMocks();
});

describe('demo DAG DB runtime adapter', () => {
  it('requires a complete DAG DB gateway configuration', () => {
    expect(() => requireDemoDagDbConfig({})).toThrow(
      /EXO_DEMO_DAGDB_GATEWAY_URL/,
    );
    expect(requireDemoDagDbConfig(COMPLETE_ENV)).toMatchObject({
      gatewayUrl: 'https://gateway.example.test',
      tenantId: 'tenant-a',
      namespace: 'demo-namespace',
    });
  });

  it('posts deterministic DAG DB intake envelopes for every SQL operation kind', async () => {
    await withProductionStore(async () => {
      globalThis.fetch = vi
        .fn()
        .mockResolvedValueOnce(okResponse({ demo_result: { rows: [{ id: 1 }], rowCount: 7 } }))
        .mockResolvedValue(okResponse({ demo_result: { rows: [{ ok: true }] } }));

      const store = createDemoServiceStore('gateway-api', { env: COMPLETE_ENV });
      const first = await store.query(
        'SELECT * FROM audit WHERE id = $1',
        ['record-1', { nested: ['b', 'a'] }],
      );
      expect(first).toEqual({ rows: [{ id: 1 }], rowCount: 7 });

      const [, firstInit] = globalThis.fetch.mock.calls[0];
      expect(globalThis.fetch.mock.calls[0][0]).toBe(
        'https://gateway.example.test/api/v1/dag-db/intake',
      );
      expect(firstInit.method).toBe('POST');
      expect(firstInit.headers).toMatchObject({
        authorization: 'Bearer demo-auth-token',
        'content-type': 'application/json',
        'x-exo-tenant-id': 'tenant-a',
        'x-exo-namespace': 'demo-namespace',
        'x-exo-authority-scope': 'dagdb:intake:tenant-a:demo-namespace',
        'x-exo-write-signature': 'signature-1',
      });

      const firstBody = JSON.parse(firstInit.body);
      expect(firstBody).toMatchObject({
        tenant_id: 'tenant-a',
        namespace: 'demo-namespace',
        requested_action: 'demo:gateway-api:select',
        consent_purpose: 'retrieval',
        owner_did: 'did:exo:owner',
        controller_did: 'did:exo:controller',
        submitted_by_did: 'did:exo:submitter',
        keyword_texts: ['demo', 'gateway-api', 'select'],
      });
      expect(firstBody.idempotency_key).toMatch(
        /^demo:gateway-api:select:[0-9a-f]{48}$/,
      );
      expect(firstBody.payload_hash).toMatch(/^[0-9a-f]{64}$/);
      expect(firstBody.source_hash).toMatch(/^[0-9a-f]{64}$/);

      for (const [sql, kind, purpose] of [
        ['WITH recent AS (SELECT 1) SELECT * FROM recent', 'select', 'retrieval'],
        ['INSERT INTO audit VALUES ($1) RETURNING id', 'returning-write', 'writeback'],
        ['INSERT INTO audit VALUES ($1)', 'insert', 'writeback'],
        ['UPDATE audit SET id = $1', 'update', 'writeback'],
        ['DELETE FROM audit WHERE id = $1', 'delete', 'writeback'],
        ['VACUUM', 'statement', 'writeback'],
      ]) {
        await store.query(sql, ['value']);
        const body = lastRequestBody();
        expect(body.requested_action).toBe(`demo:gateway-api:${kind}`);
        expect(body.consent_purpose).toBe(purpose);
        expect(body.keyword_texts).toEqual(['demo', 'gateway-api', kind]);
      }

      await expect(store.end()).resolves.toBeUndefined();
    });
  });

  it('fails closed on rejected, malformed, or incomplete DAG DB responses', async () => {
    await withProductionStore(async () => {
      const store = createDemoServiceStore('gateway-api', { env: COMPLETE_ENV });

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: false,
        status: 403,
        text: async () => 'denied',
      });
      await expect(store.query('SELECT 1')).rejects.toThrow(
        /status 403: denied/,
      );

      globalThis.fetch = vi.fn().mockResolvedValue({
        ok: true,
        status: 200,
        text: async () => 'not-json',
      });
      await expect(store.query('SELECT 1')).rejects.toThrow(
        /returned non-JSON body/,
      );

      globalThis.fetch = vi.fn().mockResolvedValue(okResponse({ demo_result: {} }));
      await expect(store.query('SELECT 1')).rejects.toThrow(
        /missing demo_result\.rows/,
      );
    });
  });

  it('keeps test stores explicit and custom query stores isolated from production config', async () => {
    const custom = createDemoServiceStore('custom-service', {
      query: async (sql, params) => ({ rows: [{ sql, params }], rowCount: 1 }),
    });
    await expect(custom.query('SELECT $1', ['x'])).resolves.toEqual({
      rows: [{ sql: 'SELECT $1', params: ['x'] }],
      rowCount: 1,
    });
    await expect(custom.end()).resolves.toBeUndefined();

    const testStore = resetDemoServiceTestStore();
    testStore.query = async () => ({ rows: [{ test: true }], rowCount: 1 });

    expect(getDemoServiceTestStore()).toBe(testStore);
    expect(createDemoServiceStore('gateway-api')).toBe(testStore);
    await expect(createDemoServiceStore('gateway-api').query('SELECT 1')).resolves.toEqual({
      rows: [{ test: true }],
      rowCount: 1,
    });
  });
});
