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
  printf 'coverage policy test failed: %s\n' "$1" >&2
  exit 1
}

[[ -f tarpaulin.toml ]] || fail "tarpaulin.toml is missing"
[[ -f .github/workflows/ci.yml ]] || fail ".github/workflows/ci.yml is missing"

python3 - <<'PY'
import sys
import tomllib
from pathlib import Path

coverage_exclusion_categories = {
    "Gateway runtime adapters": {
        "crates/exo-gateway/src/db.rs": "gateway database persistence is a core runtime adapter",
        "crates/exo-gateway/src/server.rs": "gateway HTTP authentication and routing is a core runtime adapter",
        "crates/exo-gateway/src/handlers.rs": "gateway API handlers expose core runtime decisions",
        "crates/exo-gateway/src/graphql.rs": "gateway GraphQL exposes core runtime decisions",
    },
    "DAG DB Postgres adapters": {
        "crates/exo-dag-db-postgres/src/idempotency.rs": "SQLx idempotency store requires a live Postgres schema",
        "crates/exo-dag-db-postgres/src/outbox.rs": "SQLx outbox processing requires a live Postgres schema",
        "crates/exo-dag-db-postgres/src/persistent_context.rs": "persistent context selection reads live Postgres rows",
        "crates/exo-dag-db-postgres/src/postgres/context_packet_persistence.rs": "context packet persistence requires Postgres migrations",
        "crates/exo-dag-db-postgres/src/postgres/continuation_persistence.rs": "continuation persistence requires Postgres migrations",
        "crates/exo-dag-db-postgres/src/postgres/default_route.rs": "default route persistence requires Postgres migrations",
        "crates/exo-dag-db-postgres/src/postgres/kg_catalog_router.rs": "catalog routing adapter reads live Postgres graph rows",
        "crates/exo-dag-db-postgres/src/postgres/kg_context_selection.rs": "context selection adapter reads live Postgres graph rows",
        "crates/exo-dag-db-postgres/src/postgres/kg_context_selection_write.rs": "context selection write adapter persists Postgres rows",
        "crates/exo-dag-db-postgres/src/postgres/kg_export.rs": "KG export adapter requires a live Postgres database",
        "crates/exo-dag-db-postgres/src/postgres/kg_import.rs": "KG import adapter requires a live Postgres database",
        "crates/exo-dag-db-postgres/src/postgres/kg_retrieval.rs": "KG retrieval adapter requires a live Postgres database",
        "crates/exo-dag-db-postgres/src/postgres/kg_writeback.rs": "KG writeback adapter requires a live Postgres database",
        "crates/exo-dag-db-postgres/src/postgres/lifecycle_action.rs": "lifecycle action adapter requires Postgres migrations",
        "crates/exo-dag-db-postgres/src/postgres/mod.rs": "Postgres module wiring is adapter-only",
        "crates/exo-dag-db-postgres/src/postgres/route_invalidation.rs": "route invalidation adapter requires Postgres migrations",
        "crates/exo-dag-db-postgres/src/receipt.rs": "SQLx receipt store requires a live Postgres schema",
    },
}

tarpaulin_text = Path("tarpaulin.toml").read_text()
config = tomllib.loads(tarpaulin_text)
exclude_files = config.get("default", {}).get("exclude-files", [])
excluded = set(exclude_files)

categorized_exclusions = {}
for category, paths in coverage_exclusion_categories.items():
    hits = [
        f"{path} ({reason})"
        for path, reason in paths.items()
        if path in excluded
    ]
    if hits:
        categorized_exclusions[category] = hits

if categorized_exclusions and "not a whole-workspace coverage claim" not in tarpaulin_text:
    print(
        "coverage policy test failed: structural exclusions require an "
        "explicit scoped-coverage disclosure:",
        file=sys.stderr,
    )
    for exclusions in categorized_exclusions.values():
        for exclusion in exclusions:
            print(f"  - {exclusion}", file=sys.stderr)
    sys.exit(1)

for category, exclusions in categorized_exclusions.items():
    if category not in tarpaulin_text:
        print(
            "coverage policy test failed: structural exclusions must be "
            "classified in tarpaulin.toml comments:",
            file=sys.stderr,
        )
        print(f"  - missing category: {category}", file=sys.stderr)
        for exclusion in exclusions:
            print(f"  - {exclusion}", file=sys.stderr)
        sys.exit(1)

allowed_dagdb_exclusions = set()
for category, paths in coverage_exclusion_categories.items():
    if category.startswith("DAG DB"):
        allowed_dagdb_exclusions.update(paths)

unclassified_dagdb_exclusions = [
    path
    for path in sorted(excluded)
    if (
        path.startswith("crates/exo-dag-db-")
        or path.startswith("crates/exo-gateway/src/bin/dagdb_")
    )
    and path not in allowed_dagdb_exclusions
]

if unclassified_dagdb_exclusions:
    print(
        "coverage policy test failed: DAG DB exclusions must be explicit "
        "and classified by structural untestability:",
        file=sys.stderr,
    )
    for path in unclassified_dagdb_exclusions:
        print(f"  - {path}", file=sys.stderr)
    sys.exit(1)
PY

grep -F 'bash tools/test_coverage_policy.sh' .github/workflows/ci.yml >/dev/null \
  || fail "CI must run tools/test_coverage_policy.sh before the coverage gate"

coverage_gate_block=$(
  awk '
    /# Gate 3: Coverage/ { capture = 1 }
    capture { print }
    /# Gate 4: Lint/ { exit }
  ' .github/workflows/ci.yml
)

[[ -n "$coverage_gate_block" ]] || fail "CI coverage gate block is missing"

if grep -F -- '--skip-clean' <<<"$coverage_gate_block" >/dev/null; then
  fail "default workspace coverage gate must clean instrumentation before measuring the 90% threshold"
fi

if grep -nE '90%[+[:space:]-]*line coverage|Coverage.*>=90|Coverage.*>= 90' README.md .github/workflows/ci.yml docs/guides/GETTING-STARTED.md \
  | grep -viE 'scoped|tarpaulin.toml|not independently verified outside CI' >/tmp/coverage-policy-claims.txt; then
  if grep -nE 'exclude-files|--exclude' tarpaulin.toml .github/workflows/ci.yml >/dev/null; then
    cat /tmp/coverage-policy-claims.txt >&2
    rm -f /tmp/coverage-policy-claims.txt
    fail "coverage claims must disclose that the 90% gate is scoped by explicit exclusions"
  fi
fi
rm -f /tmp/coverage-policy-claims.txt

printf 'coverage policy test passed\n'
