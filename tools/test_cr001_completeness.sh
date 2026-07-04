#!/usr/bin/env bash
# Copyright 2026 Exochain Foundation
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at:
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0

# VCG-014 governance/legal completeness guard (companion to
# tools/test_cr001_status.sh, which guards only the DRAFT status).
#
# CR-001 (AEGIS / SYBIL / Authentic Plurality) tracks a family of Sybil
# sub-threats, a set of mandatory work orders, and a release-blocking
# acceptance standard — most of which are still OPEN. The risk this guard
# closes is a SILENT completeness claim: a change that flips a Sybil
# sub-threat from TODO to done, upgrades a PARTIAL work-order row to
# IMPLEMENTED, or ticks a Section 9 release-blocking box WITHOUT the explicit
# coordinator + council review those transitions require.
#
# This guard therefore PINS the current unresolved state. It is not a claim
# that the Sybil threats are mitigated — it is machine-enforcement that no one
# can quietly assert they are. Substantive closure of each sub-threat is a
# separate future lane; when one genuinely lands, THIS guard must be updated in
# the same change set (which is exactly the review checkpoint we want).
#
# Any diff that touches BOTH this guard and CR-001 status is, by construction,
# a completeness-claim change and is flagged for coordinator + principal review.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

resolution=governance/resolutions/CR-001-AEGIS-SYBIL-AUTHENTIC-PLURALITY.md

fail() {
  echo "CR-001 completeness guard failed: $*" >&2
  exit 1
}

assert_contains() {
  local file=$1
  local pattern=$2
  if ! grep -Fq -- "$pattern" "$file"; then
    fail "$file missing required pinned line: $pattern"
  fi
}

assert_lacks_regex() {
  local file=$1
  local pattern=$2
  if grep -nE "$pattern" "$file"; then
    fail "$file contains forbidden pattern (silent completeness claim?): $pattern"
  fi
}

# ---------------------------------------------------------------------------
# 1. Section 8.2 — the six Sybil sub-threat rows must ALL remain TODO.
#    A row flipping away from TODO is a substantive-closure claim and must
#    update this guard (and be reviewed) rather than land silently.
# ---------------------------------------------------------------------------
assert_contains "$resolution" '| Identity Sybil | DID/credential layer | TODO |'
assert_contains "$resolution" '| Review Sybil | Clearance/approval pipelines | TODO |'
assert_contains "$resolution" '| Quorum Sybil | Governance voting | TODO |'
assert_contains "$resolution" '| Delegation Sybil | Authority chains | TODO |'
assert_contains "$resolution" '| Mesh Sybil | Peer discovery/networking | TODO |'
assert_contains "$resolution" '| Synthetic-Opinion Sybil | AI-generated review plurality | TODO |'

# ---------------------------------------------------------------------------
# 2. Implementation Tracking — eight work orders remain PARTIAL and exactly
#    one (8.8 release gating) is IMPLEMENTED. Pin each status caveat verbatim.
# ---------------------------------------------------------------------------
assert_contains "$resolution" '| 8.1 Spec harmonization | SPEC_GUARDIAN | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.2 Threat expansion | SECURITY_THREATS_AGENT | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.3 Provenance enforcement | Council | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.4 Clearance hardening | Council | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.5 Challenge path hardening | Council | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.6 Escalation pathway | Council | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.7 Traceability completion | QA_TDD_AGENT + SPEC_GUARDIAN | 🟡 PARTIAL |'
assert_contains "$resolution" '| 8.8 Release gating | DEVOPS_RELEASE_AGENT | ✅ IMPLEMENTED |'
assert_contains "$resolution" '| 8.9 No-admin preservation | Council | 🟡 PARTIAL |'

# ---------------------------------------------------------------------------
# 3. Section 9 — the six release-blocking acceptance boxes must remain
#    UNCHECKED. A ticked box is a constitutional-readiness claim; it may only
#    land with explicit review + a matching guard update.
# ---------------------------------------------------------------------------
assert_contains "$resolution" '- [ ] One unambiguous normative definition source for AEGIS and SYBIL'
assert_contains "$resolution" '- [ ] Threat matrix includes full Sybil family with mitigations and tests'
assert_contains "$resolution" '- [ ] Traceability matrix maps each requirement to implementation and tests'
assert_contains "$resolution" '- [ ] Plural-governance paths enforce provenance and independence-aware counting'
assert_contains "$resolution" '- [ ] Challenge and escalation flows can pause contested decisions'
assert_contains "$resolution" '- [ ] Quality gates pass without exception'

# Defense in depth: none of the six release-blocking boxes may appear in the
# ticked ([x]/[X]) form. If a future lane genuinely satisfies one, it updates
# both the resolution and this guard in the same reviewed change set.
assert_lacks_regex "$resolution" '^- \[[xX]\] One unambiguous normative definition source for AEGIS and SYBIL$'
assert_lacks_regex "$resolution" '^- \[[xX]\] Threat matrix includes full Sybil family with mitigations and tests$'
assert_lacks_regex "$resolution" '^- \[[xX]\] Traceability matrix maps each requirement to implementation and tests$'
assert_lacks_regex "$resolution" '^- \[[xX]\] Plural-governance paths enforce provenance and independence-aware counting$'
assert_lacks_regex "$resolution" '^- \[[xX]\] Challenge and escalation flows can pause contested decisions$'
assert_lacks_regex "$resolution" '^- \[[xX]\] Quality gates pass without exception$'

echo "CR-001 completeness guard passed (6 Sybil sub-threats TODO, 8 work orders PARTIAL + 1 IMPLEMENTED, 6 release-blocking boxes unchecked)"
