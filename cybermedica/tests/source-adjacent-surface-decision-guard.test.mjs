// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See cybermedica/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { test } from 'node:test';

const EXPECTED_ADJACENT_SURFACE_DECISION_IDS = Object.freeze([
  'ASD-001',
  'ASD-002',
  'ASD-003',
  'ASD-004',
  'ASD-005',
  'ASD-006',
  'ASD-007',
  'ASD-008',
  'ASD-009',
  'ASD-010',
  'ASD-011',
]);

test('source adjacent-surface decision guard enforces ASD register source and test coverage', async () => {
  const { scanAdjacentSurfaceDecisionCoverage } = await import('../scripts/source-adjacent-surface-decision-guard.mjs');
  const report = scanAdjacentSurfaceDecisionCoverage();

  assert.equal(report.schema, 'cybermedica.source_adjacent_surface_decision_guard.v1');
  assert.equal(report.status, 'passed');
  assert.equal(report.exitCode, 0);
  assert.deepEqual(report.adjacentSurfaceDecisionIds, EXPECTED_ADJACENT_SURFACE_DECISION_IDS);
  assert.equal(report.adjacentSurfaceDecisionCount, 11);
  assert.equal(report.findingsCount, 0);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.checkedSourceRefs.includes('src/service-contract-publication.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/trust-adapter.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/ci-cd-quality-gates.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/optional-trust-claim-guards.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/service-contract-publication.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/adapter-fail-closed.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/production-trust-activation.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/optional-trust-claim-guards.test.mjs'));
});

test('source adjacent-surface decision guard emits metadata-only drift findings', async () => {
  const { scanAdjacentSurfaceDecisionCoverage } = await import('../scripts/source-adjacent-surface-decision-guard.mjs');
  const fixtureRoot = mkdtempSync(join(tmpdir(), 'cybermedica-asd-guard-'));
  const contextDir = join(fixtureRoot, 'docs', 'context');
  mkdirSync(contextDir, { recursive: true });
  writeFileSync(
    join(contextDir, 'CYBERMEDICA_ADJACENT_SURFACE_DECISIONS.md'),
    `# Fixture\n\n| ID | Decision | Rationale | Source basis | Status |\n|---|---|---|---|---|\n| ASD-001 | CyberMedica remains adjacent to Exochain core. | Boundary. | AGENTS.md | Adopted |\n`,
  );

  const report = scanAdjacentSurfaceDecisionCoverage(fixtureRoot);

  assert.equal(report.status, 'failed');
  assert.equal(report.exitCode, 1);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.findingsCount > 0);
  assert.ok(report.findings.every((finding) => finding.metadataOnly === true));
  assert.ok(report.findings.some((finding) => finding.ruleId === 'adjacent_surface_decision_register_id_absent'));
  assert.ok(report.findings.some((finding) => finding.ruleId === 'adjacent_surface_decision_source_file_absent'));
});
