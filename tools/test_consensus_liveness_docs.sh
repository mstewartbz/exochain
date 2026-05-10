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

cd "$(dirname "$0")/.."

guide="docs/guides/architecture-overview.md"
consensus="crates/exo-dag/src/consensus.rs"

if grep -q "view-change on timeout" "$guide"; then
  echo "architecture guide must not claim view-change on timeout for current DAG consensus" >&2
  exit 1
fi

if grep -q "BFT-HotStuff derivative" "$guide"; then
  echo "architecture guide must not describe current DAG consensus as HotStuff-derived" >&2
  exit 1
fi

grep -q "Current liveness boundary" "$guide" || {
  echo "architecture guide must document the current consensus liveness boundary" >&2
  exit 1
}

grep -q "Liveness assumptions" "$consensus" || {
  echo "exo-dag consensus module docs must spell out liveness assumptions" >&2
  exit 1
}

grep -q "does not implement leader election or view-change" "$consensus" || {
  echo "exo-dag consensus docs must explicitly reject unstated leader/view-change behavior" >&2
  exit 1
}

echo "consensus liveness documentation test passed"
