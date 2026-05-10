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
import { execFile } from 'node:child_process';
import { equal, deepEqual, ok, rejects } from 'node:assert/strict';
import { promisify } from 'node:util';

import {
  EXOCHAIN_SETTLEMENT_AUTHORITY,
  ExoForgeHonorGoodClient,
  generateLegacyReceiptProposal,
} from '../lib/honorgood.js';

const execFileAsync = promisify(execFile);

test('legacy receipt proposals are unratified EXOCHAIN submissions', () => {
  const proposal = generateLegacyReceiptProposal({
    upstreamProject: 'Archon',
    receivingSystem: 'ExoForge',
    license: 'MIT',
    sourceUri: 'https://github.com/coleam00/Archon',
    materialityTier: 'Genesis',
    proposedBasis: [
      { basis: 'NetRevenue', share_bp: 100 },
      { basis: 'ProtocolFee', share_bp: 500 },
    ],
  });

  equal(proposal.settlement_authority, EXOCHAIN_SETTLEMENT_AUTHORITY);
  equal(proposal.local_settlement_authority, false);
  equal(proposal.legacy_receipt.status, 'Proposed');
  equal(proposal.legacy_receipt.legal_effect, 'VoluntaryRecognitionOnly');
  equal(proposal.legacy_receipt.receiving_system, 'ExoForge');
  equal(proposal.legacy_receipt.license, 'MIT');
  equal(proposal.legacy_receipt.settlement_eligible, false);
  equal(proposal.legacy_receipt.signed_contributor_acceptance_hash, null);
  equal(proposal.legacy_receipt.human_ratifier_did, null);
  ok(!JSON.stringify(proposal).includes('RatifiedAgreement'));
});

test('HonorGood client fails closed when EXOCHAIN API base URL is absent', async () => {
  const client = new ExoForgeHonorGoodClient({
    baseUrl: '',
    fetchImpl: async () => {
      throw new Error('fetch must not be called');
    },
  });

  deepEqual(client.status(), {
    configured: false,
    settlement_authority: EXOCHAIN_SETTLEMENT_AUTHORITY,
    local_settlement_authority: false,
  });
  await rejects(
    () => client.submitLegacyReceipt({ contribution_name: 'Paperclip' }),
    /EXOCHAIN_API_BASE_URL is required/,
  );
});

test('HonorGood client submits legacy receipts to EXOCHAIN core API only', async () => {
  const requests = [];
  const client = new ExoForgeHonorGoodClient({
    baseUrl: 'https://exochain.test/',
    apiToken: 'secret-token',
    fetchImpl: async (url, options) => {
      requests.push({ url, options });
      return {
        ok: true,
        status: 200,
        json: async () => ({
          object: { legacy_receipt_id: 'abc' },
          anchor: { object_kind: 'legacy_receipt' },
        }),
      };
    },
  });

  const response = await client.submitLegacyReceipt({ contribution_name: 'Paperclip' });

  equal(response.object.legacy_receipt_id, 'abc');
  equal(requests.length, 1);
  equal(requests[0].url, 'https://exochain.test/api/v1/economy/legacy-receipts');
  equal(requests[0].options.method, 'POST');
  equal(requests[0].options.headers.Authorization, 'Bearer secret-token');
  equal(JSON.parse(requests[0].options.body).contribution_name, 'Paperclip');
});

test('exoforge-honorgood propose-legacy emits JSON unratified proposal', async () => {
  const { stdout } = await execFileAsync(process.execPath, [
    'exoforge/bin/exoforge-honorgood.js',
    'propose-legacy',
    '--upstream',
    'Paperclip',
    '--receiving',
    'CommandBase',
    '--license',
    'MIT',
    '--source-uri',
    'https://github.com/paperclip-ui/paperclip',
    '--materiality',
    'Foundational',
    '--basis',
    'SoftwareArr:150',
    '--basis',
    'ImplementationFee:500',
    '--json',
  ], {
    cwd: new URL('../..', import.meta.url),
    maxBuffer: 1024 * 1024,
  });
  const report = JSON.parse(stdout);

  equal(report.settlement_authority, EXOCHAIN_SETTLEMENT_AUTHORITY);
  equal(report.legacy_receipt.status, 'Proposed');
  equal(report.legacy_receipt.receiving_system, 'CommandBase');
  equal(report.proposed_terms.basis_lines.length, 2);
  ok(!JSON.stringify(report).includes('RatifiedAgreement'));
});
