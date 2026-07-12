<!--
Copyright 2026 Exochain Foundation

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at:

    https://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

SPDX-License-Identifier: Apache-2.0
-->


# Mission C2 Progress (rollup by reference)

**Not a status ledger.** Read SSOTs:

| Concern | Source |
|---------|--------|
| VCG / gap execution | [`GAP-REGISTRY.md`](../../GAP-REGISTRY.md) |
| W1–W5 rulings | [`docs/governance/RATIFICATION-SLATE-2026-07-04.md`](../governance/RATIFICATION-SLATE-2026-07-04.md) |
| DAG DB runtime | [`INTEGRATION.md`](../../INTEGRATION.md), [`docs/dagdb/runtime-activation/rollback-canary-observability.md`](../dagdb/runtime-activation/rollback-canary-observability.md) |
| Repo metrics | `tools/repo_truth.sh` → README |
| Release | [`CHANGELOG.md`](../../CHANGELOG.md), [`VERSIONING.md`](../../VERSIONING.md) |
| Mission Graph nodes | [`mission-graph.yaml`](mission-graph.yaml) |

Guard: `tools/test_mission_c2_graph.sh` — every YAML node has a matching `nodes/<id>.md`.
