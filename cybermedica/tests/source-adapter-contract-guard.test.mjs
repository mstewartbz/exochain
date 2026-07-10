// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See cybermedica/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { test } from 'node:test';

const EXPECTED_MINIMUM_ADAPTER_REQUIREMENT_IDS = Object.freeze([
  'MAC-001',
  'MAC-002',
  'MAC-003',
  'MAC-004',
  'MAC-005',
  'MAC-006',
]);

test('source adapter-contract guard enforces integration-map minimum adapter contract coverage', async () => {
  const { scanMinimumAdapterContractCoverage } = await import('../scripts/source-adapter-contract-guard.mjs');
  const report = scanMinimumAdapterContractCoverage();

  assert.equal(report.schema, 'cybermedica.source_adapter_contract_guard.v1');
  assert.equal(report.status, 'passed');
  assert.equal(report.exitCode, 0);
  assert.deepEqual(report.minimumAdapterRequirementIds, EXPECTED_MINIMUM_ADAPTER_REQUIREMENT_IDS);
  assert.equal(report.minimumAdapterRequirementCount, 6);
  assert.equal(report.findingsCount, 0);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.checkedSourceRefs.includes('src/trust-adapter.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/gateway-call-path.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/node-receipt-sync.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/privacy-fixture-boundary.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/audit-event-receipts.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/requirement-traceability.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/service-contract-publication.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/adapter-fail-closed.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/gateway-call-path.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/node-receipt-sync.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/privacy-fixture-boundary.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/audit-event-receipts.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/requirement-traceability.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/service-contract-publication.test.mjs'));
});

test('source adapter-contract guard emits metadata-only drift findings', async () => {
  const { scanMinimumAdapterContractCoverage } = await import('../scripts/source-adapter-contract-guard.mjs');
  const fixtureRoot = mkdtempSync(join(tmpdir(), 'cybermedica-adapter-contract-'));
  const contextDir = join(fixtureRoot, 'docs', 'context');
  mkdirSync(contextDir, { recursive: true });
  writeFileSync(
    join(contextDir, 'EXOCHAIN_TO_CYBERMEDICA_INTEGRATION_MAP.md'),
    `# Fixture\n\n## Minimum Adapter Contract Tests\n\nEvery CyberMedica Exochain adapter must prove:\n\n1. It fails closed when Exochain is unavailable, returns an error, times out, rejects auth, rejects consent, rejects authority, rejects quorum, or cannot create a receipt.\n`,
  );

  const report = scanMinimumAdapterContractCoverage(fixtureRoot);

  assert.equal(report.status, 'failed');
  assert.equal(report.exitCode, 1);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.findingsCount > 0);
  assert.ok(report.findings.every((finding) => finding.metadataOnly === true));
  assert.ok(report.findings.some((finding) => finding.ruleId === 'adapter_contract_requirement_text_absent'));
  assert.ok(report.findings.some((finding) => finding.ruleId === 'adapter_contract_source_file_absent'));
});
