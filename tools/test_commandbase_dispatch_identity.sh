#!/usr/bin/env bash
# Guard CommandBase's adjacent agent fleet against anonymous auto-spawned
# terminals. Each dispatched terminal must carry a stable identity envelope so
# agent actions remain attributable without expanding EXOCHAIN core trust.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "commandbase dispatch identity test failed: $*" >&2
  exit 1
}

assert_contains() {
  local file="$1"
  local expected="$2"

  grep -Fq -- "$expected" "$file" || fail "$file must contain: $expected"
}

assert_contains command-base/CLAUDE.md "per-dispatch identity assertion"
assert_contains command-base/CLAUDE.md "dispatch_id"
assert_contains command-base/CLAUDE.md "team_member_id"
assert_contains command-base/CLAUDE.md "task_assignment_id"
assert_contains command-base/CLAUDE.md "agent_passport_id"
assert_contains command-base/CLAUDE.md "actor_did"
assert_contains command-base/CLAUDE.md "spawn must fail closed"
assert_contains command-base/CLAUDE.md "shared Board, executive, or team identity"
assert_contains command-base/CLAUDE.md "does not make the adjacent CommandBase agent an EXOCHAIN core actor"

echo "commandbase dispatch identity test passed"
