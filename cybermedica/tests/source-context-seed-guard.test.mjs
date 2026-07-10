// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See cybermedica/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { test } from 'node:test';

const EXPECTED_GUARDRAIL_NUMBERS = Object.freeze([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);

const EXPECTED_PERMITTED_PRIMITIVE_FAMILIES = Object.freeze([
  'DAG/provenance',
  'DID identity',
  'Decision Forum adjudicated workflow',
  'TrustReceipt',
  'authority chains',
  'bailment/consent',
  'gatekeeper adjudication',
  'legal evidence custody',
  'root trust bundle verification',
  'tenant registry',
  'verified quorum/governance audit',
]);

const EXPECTED_FORBIDDEN_PRODUCTION_CLAIM_FAMILIES = Object.freeze([
  '0dentity behavioral/device axes',
  'Archon workflows',
  'CommandBase',
  'CrossChecked anchoring',
  'ExoForge',
  'any UI surface',
  'default-off proofs',
  'economy settlement',
  'raw admin governance',
]);

test('source context-seed guard enforces doctrine and mapped implementation coverage', async () => {
  const { scanContextSeedDoctrineCoverage } = await import('../scripts/source-context-seed-guard.mjs');
  const report = scanContextSeedDoctrineCoverage();

  assert.equal(report.schema, 'cybermedica.source_context_seed_guard.v1');
  assert.equal(report.status, 'passed');
  assert.equal(report.exitCode, 0);
  assert.deepEqual(report.guardrailNumbers, EXPECTED_GUARDRAIL_NUMBERS);
  assert.deepEqual(report.permittedPrimitiveFamilies, EXPECTED_PERMITTED_PRIMITIVE_FAMILIES);
  assert.deepEqual(report.forbiddenProductionClaimFamilies, EXPECTED_FORBIDDEN_PRODUCTION_CLAIM_FAMILIES);
  assert.equal(report.guardrailCount, 15);
  assert.equal(report.permittedPrimitiveFamilyCount, 11);
  assert.equal(report.forbiddenProductionClaimFamilyCount, 9);
  assert.equal(report.findingsCount, 0);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.checkedSourceRefs.includes('src/ground-truth-register.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/requirement-traceability.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/service-contract-publication.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/ground-truth-register.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/requirement-traceability.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/service-contract-publication.test.mjs'));
});

test('source context-seed guard emits metadata-only drift findings', async () => {
  const { scanContextSeedDoctrineCoverage } = await import('../scripts/source-context-seed-guard.mjs');
  const fixtureRoot = mkdtempSync(join(tmpdir(), 'cybermedica-context-seed-guard-'));
  const contextDir = join(fixtureRoot, 'docs', 'context');
  mkdirSync(contextDir, { recursive: true });
  writeFileSync(
    join(contextDir, 'EXOCHAIN_CONTEXT_SEED_FOR_CYBERMEDICA.md'),
    `# Fixture

This seed permits CyberMedica to map these source-identified primitive families into baseline development service contracts now: tenant registry.

## 12. CyberMedica Guardrails

1. CyberMedica is an adjacent app, not Exochain core.
`,
  );

  const report = scanContextSeedDoctrineCoverage(fixtureRoot);

  assert.equal(report.status, 'failed');
  assert.equal(report.exitCode, 1);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.findingsCount > 0);
  assert.ok(report.findings.every((finding) => finding.metadataOnly === true));
  assert.ok(report.findings.some((finding) => finding.ruleId === 'context_seed_guardrail_absent'));
  assert.ok(report.findings.some((finding) => finding.ruleId === 'context_seed_source_file_absent'));
});
