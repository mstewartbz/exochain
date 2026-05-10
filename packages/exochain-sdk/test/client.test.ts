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

import { test } from 'node:test';
import { rejects, strictEqual } from 'node:assert/strict';
import { ExochainClient } from '../src/client.js';
import { TransportError } from '../src/errors.js';
import { validateDid } from '../src/identity/did.js';
import type { Hash256 } from '../src/types.js';

const HASH_64 = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef' as Hash256;

interface RecordedFetch {
  readonly calls: RequestInit[];
  readonly fetch: typeof fetch;
}

function jsonFetch(body: unknown, status = 200): RecordedFetch {
  const calls: RequestInit[] = [];
  const fetchImpl = (async (_input: RequestInfo | URL, init?: RequestInit) => {
    if (init !== undefined) {
      calls.push(init);
    }
    return new Response(JSON.stringify(body), {
      status,
      headers: { 'content-type': 'application/json' },
    });
  }) as typeof fetch;
  return { calls, fetch: fetchImpl };
}

test('health rejects malformed gateway payloads instead of trusting casts', async () => {
  const transport = jsonFetch({ status: 'ok', version: '0.1.0', uptime: 'not-a-number' });
  const client = new ExochainClient({
    baseUrl: 'https://gateway.example',
    fetch: transport.fetch,
  });

  await rejects(() => client.health(), TransportError);
});

test('health accepts a valid gateway payload', async () => {
  const transport = jsonFetch({ status: 'ok', version: '0.1.0', uptime: 42 });
  const client = new ExochainClient({
    baseUrl: 'https://gateway.example',
    fetch: transport.fetch,
  });

  const health = await client.health();

  strictEqual(health.status, 'ok');
  strictEqual(health.version, '0.1.0');
  strictEqual(health.uptime, 42);
});

test('identity.register rejects malformed DID response payloads', async () => {
  const transport = jsonFetch({ did: 'not-a-did' });
  const client = new ExochainClient({
    baseUrl: 'https://gateway.example',
    fetch: transport.fetch,
  });

  await rejects(
    () => client.identity.register({ id: validateDid('did:exo:alice') }),
    TransportError,
  );
});

test('mutating calls reject non-object request bodies before fetch', async () => {
  const transport = jsonFetch({ proposalId: HASH_64 });
  const client = new ExochainClient({
    baseUrl: 'https://gateway.example',
    fetch: transport.fetch,
  });

  await rejects(
    () => client.consent.proposeBailment('not-json-object' as never),
    TransportError,
  );
  strictEqual(transport.calls.length, 0);
});

test('governance.createDecision rejects malformed hash responses', async () => {
  const transport = jsonFetch({ decisionId: 'abc123' });
  const client = new ExochainClient({
    baseUrl: 'https://gateway.example',
    fetch: transport.fetch,
  });

  await rejects(
    () => client.governance.createDecision({ title: 'Ratify', proposer: 'did:exo:alice' }),
    TransportError,
  );
});

test('governance.getDecision validates optional quorum payload shape', async () => {
  const transport = jsonFetch({
    decisionId: HASH_64,
    status: 'proposed',
    quorum: {
      met: true,
      threshold: 2,
      totalVotes: 3,
      approvals: 2,
      rejections: 0,
      abstentions: 'zero',
    },
  });
  const client = new ExochainClient({
    baseUrl: 'https://gateway.example',
    fetch: transport.fetch,
  });

  await rejects(() => client.governance.getDecision(HASH_64), TransportError);
});
