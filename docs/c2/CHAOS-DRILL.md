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


# Chaos Drill (Monthly)

## Drills

1. **Slack kill** — disable primary webhook; confirm SMS secondary delivers; receipt shows Slack fail + SMS ok.
2. **Empty-evidence IRB** — inject unanimous advisories with empty evidence hashes; confirm item **never** reaches brief as binding.
3. **Dual-gate spoof** — attempt ratify with agent identity as Max; confirm `TwoPersonGateRequired`.

## Record

Write drill receipt (date, outcomes, fingerprints). Repeated failure is itself a CCIR blocker.

## Schedule

GitHub workflow `presidential-daily-attention.yml` documents monthly drill reminder; execution may be manual until Railway job exists.
