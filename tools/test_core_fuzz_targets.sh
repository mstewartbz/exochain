#!/usr/bin/env bash
# Regression guard for F-145: core parser fuzz targets must exist and compile.

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

fail() {
  echo "core fuzz target test failed: $*" >&2
  exit 1
}

[ -f fuzz/Cargo.toml ] || fail "fuzz/Cargo.toml is missing"
[ -f fuzz/Cargo.lock ] || fail "fuzz/Cargo.lock is missing"

for target in did_parse signature_cbor clearance_policy_json; do
  [ -f "fuzz/fuzz_targets/${target}.rs" ] || fail "missing fuzz target: ${target}"
  grep -F 'fuzz_target!' "fuzz/fuzz_targets/${target}.rs" >/dev/null \
    || fail "${target} does not define a cargo-fuzz fuzz_target"
done

cargo check --manifest-path fuzz/Cargo.toml --bins --locked

echo "core fuzz target test passed"
