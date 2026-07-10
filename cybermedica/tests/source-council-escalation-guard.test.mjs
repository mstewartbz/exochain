// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See cybermedica/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import assert from 'node:assert/strict';
import { test } from 'node:test';

const EXPECTED_BOB_ESCALATION_IDS = Object.freeze([
  'ESC-CONSENT-LEGAL',
  'ESC-HUMAN-PROOFING',
  'ESC-OPS-SECRETS',
  'ESC-OPTIONAL-ADJACENT',
  'ESC-ROLE-MATRIX',
  'ESC-ROOT-ARTIFACT-STORE',
  'ESC-ROOT-DEPLOYMENT',
  'ESC-ROOT-OWNER',
  'ESC-ROOT-ROSTER',
  'ESC-RUNTIME',
]);

test('source council-escalation guard enforces narrowed Bob escalation register coverage', async () => {
  const { scanCouncilEscalationCoverage } = await import('../scripts/source-council-escalation-guard.mjs');
  const report = scanCouncilEscalationCoverage();

  assert.equal(report.schema, 'cybermedica.source_council_escalation_guard.v1');
  assert.equal(report.status, 'passed');
  assert.equal(report.exitCode, 0);
  assert.deepEqual(report.allowedBobEscalationIds, EXPECTED_BOB_ESCALATION_IDS);
  assert.equal(report.allowedBobEscalationCount, 10);
  assert.equal(report.findingsCount, 0);
  assert.equal(report.exochainSourceExcluded, true);
  assert.equal(report.metadataOnly, true);
  assert.ok(report.checkedSourceRefs.includes('src/ground-truth-register.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/open-question-register.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/release-readiness-matrix.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/sandy-review-register.mjs'));
  assert.ok(report.checkedSourceRefs.includes('src/scope-legal-review-register.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/ground-truth-register.test.mjs'));
  assert.ok(report.checkedTestRefs.includes('tests/open-question-register.test.mjs'));
});

