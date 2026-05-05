#!/usr/bin/env bash
# Guard deployment contracts against probing the liveness-only /health route.
# Production rollout checks must use /ready so dependency failures stop deploys.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "deployment readiness probe test failed: $*" >&2
  exit 1
}

assert_file_contains() {
  local file="$1"
  local expected="$2"

  grep -Fq -- "$expected" "$file" || fail "$file must contain: $expected"
}

assert_healthcheck_uses_ready() {
  local file="$1"

  assert_file_contains "$file" "HEALTHCHECK"
  assert_file_contains "$file" "/ready"

  if awk '
    /HEALTHCHECK/ { in_healthcheck = 1 }
    in_healthcheck && /\/health/ { found = 1 }
    in_healthcheck && !/\\$/ && /exit 1/ { in_healthcheck = 0 }
    END { exit found ? 0 : 1 }
  ' "$file"; then
    fail "$file HEALTHCHECK must not probe /health"
  fi
}

assert_healthcheck_uses_ready Dockerfile
assert_healthcheck_uses_ready deploy/Dockerfile.node
assert_file_contains railway.json '"healthcheckPath": "/ready"'
assert_file_contains deploy/fly.toml 'path = "/ready"'

echo "deployment readiness probe test passed"
