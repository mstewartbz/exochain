#!/usr/bin/env bash
set -euo pipefail

fail() {
  printf 'ci supply-chain hardening test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/ci.yml"
[[ -f "$workflow" ]] || fail "$workflow is missing"

if grep -nE 'curl[^\n]*(\|[[:space:]]*(sh|bash)|>[[:space:]]*/tmp/)' "$workflow"; then
  fail "GitHub Actions workflow must not install tools through curl-piped shell scripts"
fi

install_block=$(
  awk '
    /name: Install wasm-pack/ { capture = 1 }
    capture { print }
    capture && /name: Build WASM/ { exit }
  ' "$workflow"
)

[[ -n "$install_block" ]] || fail "CI workflow must include an Install wasm-pack step"

grep -F 'cargo install wasm-pack' <<<"$install_block" >/dev/null \
  || fail "wasm-pack must be installed through Cargo, not a shell installer"

grep -E -- '--version[[:space:]]+[0-9]+\.[0-9]+\.[0-9]+' <<<"$install_block" >/dev/null \
  || fail "wasm-pack Cargo install must pin an explicit version"

grep -F -- '--locked' <<<"$install_block" >/dev/null \
  || fail "wasm-pack Cargo install must use --locked for dependency resolution"

printf 'ci supply-chain hardening test passed\n'
