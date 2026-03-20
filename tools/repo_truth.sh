#!/usr/bin/env bash
# EXOCHAIN Repository Truth Generator
# Produces machine-verifiable facts about the repository state.
# Used to validate public claims in README.md and docs.
#
# Usage: bash tools/repo_truth.sh [--json]

set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

JSON_MODE=false
if [[ "${1:-}" == "--json" ]]; then
  JSON_MODE=true
fi

# ── Counts ──
CRATE_COUNT=$(ls -d crates/*/ 2>/dev/null | wc -l | tr -d ' ')
RS_FILE_COUNT=$(find crates -name '*.rs' 2>/dev/null | wc -l | tr -d ' ')
RS_LOC=$(find crates -name '*.rs' -exec cat {} + 2>/dev/null | wc -l | tr -d ' ')
TLA_COUNT=$(find tla -name '*.tla' 2>/dev/null | wc -l | tr -d ' ')

# ── Test count (parse from last cargo test or run fresh) ──
TEST_OUTPUT=$(cargo test --workspace --lib 2>&1 || true)
TESTS_PASSED=$(echo "$TEST_OUTPUT" | grep '^test result:' | awk '{sum+=$4} END {print sum+0}')
TESTS_FAILED=$(echo "$TEST_OUTPUT" | grep '^test result:' | awk '{sum+=$6} END {print sum+0}')

# ── Traceability ──
TRACE_GREEN=$(grep -c '🟢' governance/traceability_matrix.md 2>/dev/null || echo 0)
TRACE_YELLOW=$(grep -c '🟡' governance/traceability_matrix.md 2>/dev/null || echo 0)
TRACE_RED=$(grep -c '🔴' governance/traceability_matrix.md 2>/dev/null || echo 0)
TRACE_TOTAL=$((TRACE_GREEN + TRACE_YELLOW + TRACE_RED))

# ── Threat model ──
THREAT_GREEN=$(grep -c '🟢' governance/threat_matrix.md 2>/dev/null || echo 0)
THREAT_YELLOW=$(grep -c '🟡' governance/threat_matrix.md 2>/dev/null || echo 0)
THREAT_RED=$(grep -c '🔴' governance/threat_matrix.md 2>/dev/null || echo 0)
THREAT_TOTAL=$((THREAT_GREEN + THREAT_YELLOW + THREAT_RED))

# ── License ──
CARGO_LICENSE=$(grep '^license' Cargo.toml | head -1 | sed 's/license = "//;s/"//')
LICENSE_FILE=$(head -1 LICENSE 2>/dev/null || echo "MISSING")
README_LICENSE=$(grep -oP 'Apache-2\.0|AGPL-3\.0|MIT' README.md | head -1 || echo "NONE")

# ── Release state ──
TAG_COUNT=$(git tag -l | wc -l | tr -d ' ')
HAS_CHANGELOG=$([[ -f CHANGELOG.md ]] && echo "true" || echo "false")
HAS_SECURITY=$([[ -f SECURITY.md ]] && echo "true" || echo "false")

# ── Governance artifacts ──
RESOLUTION_COUNT=$(ls governance/resolutions/*.md 2>/dev/null | grep -v INDEX | wc -l | tr -d ' ')
GOVERNANCE_DOCS=$(ls governance/*.md 2>/dev/null | wc -l | tr -d ' ')

# ── Build checks (quick, no full rebuild) ──
FMT_OK=$(cargo +nightly fmt --all -- --check 2>&1 && echo "true" || echo "false")

# ── Output ──
TIMESTAMP=$(date -u +%Y-%m-%dT%H:%M:%SZ)
COMMIT=$(git rev-parse --short HEAD)

if $JSON_MODE; then
cat <<JEOF
{
  "timestamp": "$TIMESTAMP",
  "commit": "$COMMIT",
  "crates": $CRATE_COUNT,
  "rust_source_files": $RS_FILE_COUNT,
  "rust_loc": $RS_LOC,
  "tla_specs": $TLA_COUNT,
  "tests_passed": $TESTS_PASSED,
  "tests_failed": $TESTS_FAILED,
  "traceability": { "implemented": $TRACE_GREEN, "partial": $TRACE_YELLOW, "planned": $TRACE_RED, "total": $TRACE_TOTAL },
  "threats": { "mitigated": $THREAT_GREEN, "partial": $THREAT_YELLOW, "planned": $THREAT_RED, "total": $THREAT_TOTAL },
  "license": { "cargo_toml": "$CARGO_LICENSE", "license_file": "$LICENSE_FILE", "readme": "$README_LICENSE" },
  "releases": { "tag_count": $TAG_COUNT, "has_changelog": $HAS_CHANGELOG, "has_security_md": $HAS_SECURITY },
  "governance": { "resolutions": $RESOLUTION_COUNT, "governance_docs": $GOVERNANCE_DOCS },
  "fmt_clean": $FMT_OK
}
JEOF
else
cat <<TEOF
EXOCHAIN Repository Truth — $TIMESTAMP (commit $COMMIT)
═══════════════════════════════════════════════════════

Crates:              $CRATE_COUNT
Rust source files:   $RS_FILE_COUNT
Rust LOC:            $RS_LOC
TLA+ specs:          $TLA_COUNT

Tests passed:        $TESTS_PASSED
Tests failed:        $TESTS_FAILED

Traceability:        $TRACE_GREEN implemented / $TRACE_YELLOW partial / $TRACE_RED planned (of $TRACE_TOTAL)
Threats:             $THREAT_GREEN mitigated / $THREAT_YELLOW partial / $THREAT_RED planned (of $THREAT_TOTAL)

License (Cargo.toml):  $CARGO_LICENSE
License (LICENSE file): $LICENSE_FILE
License (README):       $README_LICENSE

Git tags:            $TAG_COUNT
CHANGELOG.md:        $HAS_CHANGELOG
SECURITY.md:         $HAS_SECURITY

Resolutions:         $RESOLUTION_COUNT
Governance docs:     $GOVERNANCE_DOCS

Format clean:        $FMT_OK
TEOF
fi
