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
  printf 'cross-platform target cache boundary test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/ci.yml"
[[ -f "$workflow" ]] || fail "$workflow is missing"

cross_platform_block=$(
  awk '
    $0 == "  cross-platform:" { capture = 1; print; next }
    capture && $0 ~ /^  [A-Za-z0-9_-]+:$/ { exit }
    capture { print }
  ' "$workflow"
)
[[ -n "$cross_platform_block" ]] || fail "Gate 16 cross-platform job is missing"

grep -F 'CARGO_TARGET_DIR=target/gate16-${{ runner.os }}-${{ runner.arch }}-${{ matrix.target }}' <<<"$cross_platform_block" >/dev/null \
  || fail "Gate 16 must isolate CARGO_TARGET_DIR by runner OS, runner architecture, and target triple"
grep -F 'key: gate16-${{ runner.os }}-${{ runner.arch }}-${{ matrix.target }}' <<<"$cross_platform_block" >/dev/null \
  || fail "Gate 16 rust-cache key must include runner OS, runner architecture, and target triple"
grep -F 'workspaces: ". -> target/gate16-${{ runner.os }}-${{ runner.arch }}-${{ matrix.target }}"' <<<"$cross_platform_block" >/dev/null \
  || fail "Gate 16 rust-cache workspace must match the isolated CARGO_TARGET_DIR"
grep -F 'path: target/gate16-${{ runner.os }}-${{ runner.arch }}-${{ matrix.target }}/${{ matrix.target }}/release/exochain' <<<"$cross_platform_block" >/dev/null \
  || fail "Gate 16 artifact upload must read from the isolated target directory"

grep -F 'bash tools/test_cross_platform_target_cache_boundary.sh' "$workflow" >/dev/null \
  || fail "Gate 9 repo hygiene must run the Gate 16 cache boundary guard"

printf 'cross-platform target cache boundary test passed\n'
