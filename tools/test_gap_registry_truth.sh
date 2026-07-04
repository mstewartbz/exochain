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

# Guard GAP-REGISTRY.md as the single systemic-integrity execution ledger.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "GAP registry truth test failed: $*" >&2
  exit 1
}

assert_contains() {
  local file=$1
  local pattern=$2
  if ! grep -Eq "$pattern" "$file"; then
    fail "$file missing required pattern: $pattern"
  fi
}

assert_lacks() {
  local file=$1
  local pattern=$2
  if grep -nE "$pattern" "$file"; then
    fail "$file contains forbidden pattern: $pattern"
  fi
}

assert_no_matches() {
  local description=$1
  shift
  local matches
  matches=$("$@" || true)
  if [[ -n "$matches" ]]; then
    printf '%s\n' "$matches" >&2
    fail "$description"
  fi
}

assert_no_matches "critical gaps must be tracked only in GAP-REGISTRY.md" \
  find . -maxdepth 1 -name 'GAP-REGISTRY-*.md'

if [[ -d docs/superpowers/plans ]]; then
  assert_no_matches "systemic-integrity plans must be consolidated into GAP-REGISTRY.md" \
    find docs/superpowers/plans -maxdepth 1 -name '*systemic-integrity*.md'
fi

if [[ -e EXOCHAIN_REAL_PROOF_BACKEND_HEAVY_LIFT_PLAN.md ]]; then
  fail "proof backend plan must be consolidated into GAP-REGISTRY.md"
fi

assert_contains GAP-REGISTRY.md "^# EXOCHAIN Systemic Integrity Ledger$"
assert_contains GAP-REGISTRY.md "\\*\\*Authority:\\*\\* single source of truth"
assert_contains GAP-REGISTRY.md "^## Single Source Rule$"
assert_contains GAP-REGISTRY.md "^## Execution Protocol$"
assert_contains GAP-REGISTRY.md "^## Execution Board$"
assert_contains GAP-REGISTRY.md "^## System Closure Gate$"
assert_contains GAP-REGISTRY.md "bash tools/test_gap_registry_truth.sh"

for id in \
  VCG-001 VCG-002 VCG-003 VCG-004 VCG-005 VCG-006 VCG-007 \
  VCG-008 VCG-009 VCG-010 VCG-011 VCG-012 VCG-013 VCG-014 VCG-015
do
  heading_count=$(grep -c "^## $id -" GAP-REGISTRY.md)
  if [[ "$heading_count" != "1" ]]; then
    fail "$id must have exactly one detailed ledger section"
  fi
  assert_contains GAP-REGISTRY.md "\\| $id \\|"
done

assert_contains GAP-REGISTRY.md "VCG-001 \\| P0 \\| Red"
assert_contains GAP-REGISTRY.md "VCG-002 \\| P0 \\| Green-local"
assert_contains GAP-REGISTRY.md "VCG-003 \\| P0 \\| Red"
assert_contains GAP-REGISTRY.md "VCG-004 \\| P0 \\| Red"
assert_contains GAP-REGISTRY.md "VCG-005 \\| P1 \\| Green-local"
assert_contains GAP-REGISTRY.md "VCG-006 \\| P1 \\| Green-local"
assert_contains GAP-REGISTRY.md "VCG-008 \\| P1 \\| Green-local"
assert_contains GAP-REGISTRY.md "VCG-012 \\| P2 \\| Green-local"
assert_contains GAP-REGISTRY.md "VCG-013 \\| P2 \\| Green-local"
assert_contains GAP-REGISTRY.md "VCG-014 \\| P2 \\| Green-local"
assert_contains GAP-REGISTRY.md "eDiscovery export is not an open origin-main gap"
assert_contains GAP-REGISTRY.md "No VCG row is closed at ledger creation"

assert_lacks GAP-REGISTRY.md "T[O]DO|future[[:space:]-]+phase|postpone|synthesize"

# Cargo package names: gate commands must use the exochain-* crates.io
# namespace. Directory names under crates/ keep exo-*; commands must not.
assert_lacks GAP-REGISTRY.md "[-]p exo-(node|gateway|gatekeeper|core|tenant|governance|proofs)([^-a-z]|$)"
assert_contains GAP-REGISTRY.md "cargo test -p exochain-proofs"
assert_contains GAP-REGISTRY.md "cargo test -p exochain-node mcp"
assert_contains GAP-REGISTRY.md "cargo test -p exochain-gatekeeper tee"
assert_contains GAP-REGISTRY.md "cargo test -p exochain-core hlc"
assert_contains GAP-REGISTRY.md "cargo test -p exochain-tenant"

# Amended-ledger anchors: released source snapshot, ratified decisions,
# doctrine line, and the claim frame that binds VCG-002.
assert_contains GAP-REGISTRY.md "2d4baec1fc84bc5e71a9d5b9d1c35e7bff4aeee1"
assert_contains GAP-REGISTRY.md "^## Ratified Decisions$"
assert_contains GAP-REGISTRY.md "ratification precedes authority; authority follows evidence"
assert_contains GAP-REGISTRY.md "invariant, adversary, evidence, detection, failure"

# Every VCG section keeps the full row shape (15 rows: VCG-001..VCG-015).
for label in "Evidence:" "Failure mode:" "Next red test:" "Remediation track:" "Closure gate:"; do
  label_count=$(grep -c "^${label}$" GAP-REGISTRY.md)
  if [[ "$label_count" != "15" ]]; then
    fail "expected 15 '${label}' sections, found ${label_count}"
  fi
done

# VCG-004's generic gate commands pass on untouched main; closure requires the
# named red tests, and the ledger must keep saying so.
assert_contains GAP-REGISTRY.md "necessary but not"
assert_contains GAP-REGISTRY.md "mcp_mutation_effect"
assert_contains GAP-REGISTRY.md "cgr_proof_fail_closed"

echo "GAP registry truth test passed"
