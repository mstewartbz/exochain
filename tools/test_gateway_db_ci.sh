#!/usr/bin/env bash
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  printf 'gateway DB CI test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/ci.yml"
[[ -f "$workflow" ]] || fail "$workflow is missing"

grep -F 'DATABASE_URL: postgres://exochain:test@localhost:5432/exochain_test' "$workflow" >/dev/null \
  || fail "Gate 13 must run with an explicit live PostgreSQL DATABASE_URL"

grep -F 'cargo test -p exo-gateway --lib --features production-db' "$workflow" >/dev/null \
  || fail "Gate 13 must run exo-gateway DB-backed library tests"

grep -F -- '--test-threads=1' "$workflow" >/dev/null \
  || fail "DB-backed gateway library tests must run serially against the shared CI database"

grep -F "cargo test --workspace --test '*' --features exo-gateway/production-db" "$workflow" >/dev/null \
  || fail "Gate 13 must retain workspace DB-backed integration tests"

printf 'gateway DB CI test passed\n'
