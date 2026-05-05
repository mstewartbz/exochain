#!/usr/bin/env bash
# Guard production container builds so deployed gateway paths compile with
# DB-backed adjudication instead of the default WO-009 deny-all scaffold.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "docker production-db feature test failed: $*" >&2
  exit 1
}

assert_file_contains() {
  local file="$1"
  local expected="$2"

  grep -Fq -- "$expected" "$file" || fail "$file must contain: $expected"
}

assert_exochain_build_enables_production_db() {
  local file="$1"

  if ! awk '
    /^RUN[[:space:]]+cargo build/ {
      command = $0
      while (command ~ /\\$/ && getline continuation) {
        sub(/\\$/, "", command)
        command = command " " continuation
      }
      if (command ~ /--bin[[:space:]]+exochain/) {
        found = 1
        if (command !~ /--features[[:space:]][^\\n]*exo-gateway\/production-db/) {
          bad = 1
        }
      }
    }
    END {
      if (!found || bad) {
        exit 1
      }
    }
  ' "$file"; then
    fail "$file must build exochain with --features exo-gateway/production-db"
  fi
}

assert_exochain_build_enables_production_db Dockerfile
assert_exochain_build_enables_production_db deploy/Dockerfile.node
assert_file_contains Dockerfile "--bin exo-gateway"

echo "docker production-db feature test passed"
