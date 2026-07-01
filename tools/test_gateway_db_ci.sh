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

grep -F 'cargo test -p exochain-gateway --lib --features production-db' "$workflow" >/dev/null \
  || fail "Gate 13 must run exo-gateway DB-backed library tests"

grep -F 'EXO_DAGDB_TEST_DATABASE_URL: ${{ env.DATABASE_URL }}' "$workflow" >/dev/null \
  || fail "Gate 13 must pass its live PostgreSQL URL to DAG DB migration regressions"

grep -F 'cargo test -p exochain-dag-db-postgres --features postgres --test migration_contract pr708_migrator_upgrades_from_last_successful_deployed_ledger' "$workflow" >/dev/null \
  || fail "Gate 13 must run the DAG DB PR #708 upgrade regression"

grep -F -- '--test-threads=1' "$workflow" >/dev/null \
  || fail "DB-backed gateway library tests must run serially against the shared CI database"

grep -F "cargo test --workspace --test '*' --features exochain-gateway/production-db" "$workflow" >/dev/null \
  || fail "Gate 13 must retain workspace DB-backed integration tests"

printf 'gateway DB CI test passed\n'
