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
  printf 'release publish boundary test failed: %s\n' "$1" >&2
  exit 1
}

workflow=".github/workflows/release.yml"
[[ -f "$workflow" ]] || fail "$workflow is missing"

job_block() {
  local job="$1"
  awk -v job="  ${job}:" '
    $0 == job { capture = 1; print; next }
    capture && $0 ~ /^  [A-Za-z0-9_-]+:$/ { exit }
    capture { print }
  ' "$workflow"
}

publish_block=$(job_block "publish")
github_release_block=$(job_block "github-release")

[[ -n "$publish_block" ]] || fail "publish job is missing"
[[ -n "$github_release_block" ]] || fail "github-release job is missing"

grep -F 'if: ${{ !inputs.dry_run }}' <<<"$publish_block" >/dev/null \
  || fail "publish job must be skipped for dry-run releases"
grep -F 'if: ${{ !inputs.dry_run }}' <<<"$github_release_block" >/dev/null \
  || fail "github-release job must be skipped for dry-run releases"

grep -F 'CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}' <<<"$publish_block" >/dev/null \
  || fail "publish job must use the crates.io registry token"
grep -F 'cargo publish -p "$crate" --allow-dirty' <<<"$publish_block" >/dev/null \
  || fail "publish job must publish every crate in the dependency-ordered loop"

if grep -E 'cargo publish.*\|\|' <<<"$publish_block" >/dev/null; then
  fail "cargo publish failures must fail the publish job"
fi

grep -E 'needs:.*publish' <<<"$github_release_block" >/dev/null \
  || fail "github-release must depend on successful crates.io publication"

printf 'release publish boundary test passed\n'
