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

cd "$(git rev-parse --show-toplevel)"

out=$(mktemp)
trap 'rm -f "$out"' EXIT

cargo deny check >"$out" 2>&1
duplicates=$(rg -c '^warning\[duplicate\]' "$out" || true)

max_duplicates=24
if [ "$duplicates" -gt "$max_duplicates" ]; then
  cat "$out" >&2
  echo "duplicate dependency warning count $duplicates exceeds cap $max_duplicates" >&2
  exit 1
fi

echo "dependency hygiene check passed: $duplicates duplicate warnings (cap $max_duplicates)"
