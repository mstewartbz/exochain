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


# Daily Attention Protocol

## Purpose

Proactive daily (and on-demand) orchestrator that greets the Executive Chairman with **only** decisions requiring presidential attention.

## Pipeline

1. **Scan** Mission Graph refs, GAP human-gate rows, Decision Forum human-gate items, Archon escalations, CQI above threshold.
2. **Filter** to Strategic/Constitutional; Operational only if `requires_human_ratification` or post-council deadlock.
3. **Council pre-route** — if Decision Forum routing incomplete, enqueue council; **do not** put on brief yet.
4. **AI-IRB** — active providers with role-differentiated manifests; record advisories + mandatory dissent.
5. **Crosscheck receipts** — bind advisories to receipt IDs ([CROSSCHECK-RECEIPTS.md](CROSSCHECK-RECEIPTS.md)); fail closed if missing.
6. **Emit brief** — summary, council disposition, per-provider advisories, receipt IDs, recommended action, challenge hooks.
7. **Actions:** inquire | challenge | ratify | veto (dual gate when irreversible).
8. **Blockers** run in parallel via [CHAIRMAN-ESCALATION.md](CHAIRMAN-ESCALATION.md) — never wait for next brief.

## Brief schema (JSON)

See [`schemas/daily-brief.schema.json`](schemas/daily-brief.schema.json).

## Empty brief

Valid outcome: `items: []` with greeting `"No presidential decisions today."`

## Automation gate

Do not enable Railway/Slack automation until [DOGFOOD.md](DOGFOOD.md) rehearsal calibrates CCIRs.
