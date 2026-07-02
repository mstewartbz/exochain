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

# VCG-002 claim-integrity guard (GAP-REGISTRY.md, Systemic Integrity Ledger).
#
# While VCG-001 is open, public proof-maturity claims must not outrun
# crates/exo-proofs reality: every SNARK/STARK/ZKML/zero-knowledge mention in
# the named public claim surfaces must carry a nearby qualifier, and living
# command surfaces must use exochain-* package names. Historical records
# (docs/audit/, docs/proof/, dated validation reports) are exempt and are
# never rewritten.
#
# Coordinator-authored (SEAT-000). Workers carry this file byte-exact; any
# edit to it in a lane diff is auto-flagged per anti-false-closure rule 5.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "systemic integrity claim check failed: $*" >&2
  exit 1
}

# A proof-system mention is "qualified" when the same line or the two lines
# on either side disclose non-production status. Keep this list tight: adding
# a euphemism here to make a claim pass is the gamed-guard vector.
QUALIFIER='pedagogical|[Uu]naudited|UNAUDITED|not production|not production-grade|NOT cryptographically|hash-based (simulation|stand-in)|skeleton|refus(al|es|ed)|fail-closed|fails closed|structural(ly)? only|not a (sound|real)'

CLAIM_FILES=(
  README.md
  governance/sub_agents.md
  governance/traceability_matrix.md
  docs/reference/CRATE-REFERENCE.md
  docs/ASI-REPORT-FEATURE.md
)

for f in "${CLAIM_FILES[@]}"; do
  [[ -f "$f" ]] || fail "$f is missing"
  while IFS=: read -r ln _; do
    [[ -n "$ln" ]] || continue
    start=$(( ln > 2 ? ln - 2 : 1 ))
    ctx=$(sed -n "${start},$(( ln + 2 ))p" "$f")
    if ! grep -Eq "$QUALIFIER" <<<"$ctx"; then
      fail "$f:$ln mentions SNARK/STARK/ZKML/zero-knowledge with no qualifier within two lines (see GAP-REGISTRY.md VCG-002)"
    fi
  done < <(grep -nE 'SNARK|STARK|ZKML|zkML|zero-knowledge' "$f" || true)
done

# The literal drifted completion claims can never reappear.
for f in governance/sub_agents.md governance/traceability_matrix.md; do
  if grep -nE 'proof systems and verifier infrastructure (are )?complete' "$f"; then
    fail "$f claims proof-system completion while VCG-001 is open"
  fi
done

# "Formal proofs" (docs/proofs/CONSTITUTIONAL-PROOFS.md, protocol state-machine
# proofs) must not be conflated with SNARK/STARK/ZKML cryptography in the same
# sentence without disambiguation.
if grep -nE 'formal proofs?.{0,80}(SNARK|STARK|ZKML)|((SNARK|STARK|ZKML).{0,80}formal proofs?)' docs/ASI-REPORT-FEATURE.md README.md 2>/dev/null | grep -vE 'protocol|state-machine|distinct|separate|not.{0,20}(SNARK|cryptograph)'; then
  fail "formal-proof language conflated with SNARK/STARK/ZKML claims; disambiguate per VCG-002"
fi

# Living command surfaces must use exochain-* package names (crates.io
# namespace, commit f5191f40). Directory paths under crates/ keep exo-*;
# cargo -p arguments must not.
LIVING_SURFACES=(
  README.md
  docs/guides
  docs/reference
  docs/benchmarks
  governance/traceability_matrix.md
  governance/threat_matrix.md
  governance/quality_gates.md
)
if grep -rnE '[-]p exo-[a-z][a-z-]*' "${LIVING_SURFACES[@]}" 2>/dev/null; then
  fail "living docs cite old exo-* package names in cargo commands; use exochain-* (historical records under docs/audit/ and docs/proof/ are exempt and stay as written)"
fi

echo "systemic integrity claim check passed"
