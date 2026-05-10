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

'use strict';

const test = require('node:test');
const assert = require('node:assert/strict');
const { ExochainEconomyClient } = require('./honorgood-economy');

test('HonorGood economy client fails closed without EXOCHAIN_API_BASE_URL', async () => {
  const client = new ExochainEconomyClient({ baseUrl: '', fetch: async () => new Response('{}') });
  assert.equal(client.status().configured, false);
  await assert.rejects(
    () => client.createMission({}),
    /EXOCHAIN_API_BASE_URL is required/,
  );
});

test('HonorGood economy client posts mission registration to EXOCHAIN', async () => {
  const calls = [];
  const client = new ExochainEconomyClient({
    baseUrl: 'https://exochain.example/',
    token: 'redacted-test-token',
    fetch: async (url, init) => {
      calls.push({ url, init });
      return new Response(JSON.stringify({ object: { mission_id: 'abc' }, anchor: {} }), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      });
    },
  });

  const result = await client.createMission({ name: 'mission' });

  assert.equal(calls[0].url, 'https://exochain.example/api/v1/economy/missions');
  assert.equal(calls[0].init.method, 'POST');
  assert.equal(calls[0].init.headers.authorization, 'Bearer redacted-test-token');
  assert.deepEqual(JSON.parse(calls[0].init.body), { name: 'mission' });
  assert.equal(result.object.mission_id, 'abc');
});
