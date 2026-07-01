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
  printf 'release reusable-CI boundary test failed: %s\n' "$1" >&2
  exit 1
}

release_workflow=".github/workflows/release.yml"
ci_workflow=".github/workflows/ci.yml"

[[ -f "$release_workflow" ]] || fail "$release_workflow is missing"
[[ -f "$ci_workflow" ]] || fail "$ci_workflow is missing"

grep -F 'uses: ./.github/workflows/ci.yml' "$release_workflow" >/dev/null \
  || fail "release workflow must keep the full CI pipeline as an explicit local workflow dependency"

grep -E '^[[:space:]]+workflow_call:[[:space:]]*$' "$ci_workflow" >/dev/null \
  || fail "ci workflow must expose on.workflow_call before release.yml can call it"

grep -F 'bash tools/test_release_reusable_ci_boundary.sh' "$ci_workflow" >/dev/null \
  || fail "CI repo hygiene must run the release reusable-CI boundary guard"

printf 'release reusable-CI boundary test passed\n'
