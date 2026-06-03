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

fail() {
  printf 'ci supply-chain hardening test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/ci.yml"
release_workflow=".github/workflows/release.yml"
retry_helper="tools/ci_cargo_retry.sh"
[[ -f "$workflow" ]] || fail "$workflow is missing"
[[ -f "$release_workflow" ]] || fail "$release_workflow is missing"
[[ -f "$retry_helper" ]] || fail "$retry_helper is missing"

if grep -nE 'curl[^\n]*(\|[[:space:]]*(sh|bash)|>[[:space:]]*/tmp/)' "$workflow"; then
  fail "GitHub Actions workflow must not install tools through curl-piped shell scripts"
fi

for checked_workflow in "$workflow" "$release_workflow"; do
  grep -F 'CARGO_NET_RETRY: "10"' "$checked_workflow" >/dev/null \
    || fail "$checked_workflow must set CARGO_NET_RETRY for registry fetch resilience"
  grep -F 'CARGO_HTTP_TIMEOUT: "120"' "$checked_workflow" >/dev/null \
    || fail "$checked_workflow must set CARGO_HTTP_TIMEOUT for registry fetch resilience"
  grep -F 'CARGO_HTTP_MULTIPLEXING: "false"' "$checked_workflow" >/dev/null \
    || fail "$checked_workflow must disable Cargo HTTP multiplexing for CI fetch resilience"
done

if grep -nE 'run:[[:space:]]+cargo build[[:space:]]+' "$workflow" "$release_workflow"; then
  fail "cargo build steps must use $retry_helper"
fi

if grep -nE 'run:[[:space:]]+cargo install[[:space:]]+' "$workflow" "$release_workflow"; then
  fail "cargo install steps must use $retry_helper"
fi

if grep -nE 'run:[[:space:]]+wasm-pack build[[:space:]]+' "$workflow" "$release_workflow"; then
  fail "wasm-pack build steps must use $retry_helper"
fi

cargo_install_lines=0
for checked_workflow in "$workflow" "$release_workflow"; do
  while IFS=: read -r line_no line; do
    [[ -n "$line_no" ]] || continue
    cargo_install_lines=$((cargo_install_lines + 1))
    if ! grep -Fq -- "$retry_helper cargo install" <<<"$line"; then
      fail "$checked_workflow:$line_no cargo install must use $retry_helper: $line"
    fi
    if ! grep -Eq -- '--version[[:space:]]+[0-9]+\.[0-9]+\.[0-9]+([[:space:]]|$)' <<<"$line"; then
      fail "$checked_workflow:$line_no cargo install must pin an explicit x.y.z --version: $line"
    fi
    if ! grep -Fq -- '--locked' <<<"$line"; then
      fail "$checked_workflow:$line_no cargo install must use --locked: $line"
    fi
  done < <(grep -nE 'cargo install[[:space:]]+' "$checked_workflow" || true)
done

[[ "$cargo_install_lines" -gt 0 ]] || fail "expected workflows to contain Cargo tool installs"

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
