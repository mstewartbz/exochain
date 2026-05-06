#!/usr/bin/env bash
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

runtime_adapter_paths = {
    "crates/exo-gateway/src/db.rs": "gateway database persistence is a core runtime adapter",
    "crates/exo-gateway/src/server.rs": "gateway HTTP authentication and routing is a core runtime adapter",
    "crates/exo-gateway/src/handlers.rs": "gateway API handlers expose core runtime decisions",
    "crates/exo-gateway/src/graphql.rs": "gateway GraphQL exposes core runtime decisions",
}

tarpaulin_text = Path("tarpaulin.toml").read_text()
config = tomllib.loads(tarpaulin_text)
exclude_files = config.get("default", {}).get("exclude-files", [])
excluded = set(exclude_files)

runtime_adapter_exclusions = [
    f"{path} ({reason})"
    for path, reason in runtime_adapter_paths.items()
    if path in excluded
]

if runtime_adapter_exclusions and "not a whole-workspace coverage claim" not in tarpaulin_text:
    print(
        "coverage policy test failed: runtime adapter exclusions require an "
        "explicit scoped-coverage disclosure:",
        file=sys.stderr,
    )
    for exclusion in runtime_adapter_exclusions:
        print(f"  - {exclusion}", file=sys.stderr)
    sys.exit(1)

if runtime_adapter_exclusions and "Gateway runtime adapters" not in tarpaulin_text:
    print(
        "coverage policy test failed: runtime adapter exclusions must be "
        "classified in tarpaulin.toml comments",
        file=sys.stderr,
    )
    sys.exit(1)
PY

grep -F 'bash tools/test_coverage_policy.sh' .github/workflows/ci.yml >/dev/null \
  || fail "CI must run tools/test_coverage_policy.sh before the coverage gate"

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
