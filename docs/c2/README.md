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

# EXOCHAIN Mission C2 — Presidential Attention Doctrine

**Classification:** Governance / C2 steering artifacts. Not a trust-claiming product UI and not constitutional authority by proximity.

## WHY vs HOW

| Layer | Owner | Content |
|-------|-------|---------|
| **WHY** | Executive Chairman (Bob) + dual gate with Max when irreversible | Mission objectives, priorities, kill criteria, ratify/veto |
| **WHAT** | Mission Graph | Workstream nodes, dependencies, acceptance gates, CCIRs |
| **HOW** | Agents / Archon / ExoForge under ratified intent | Implementation, tests, PRs |
| **EVIDENCE** | Existing SSOTs only | `GAP-REGISTRY.md`, INTEGRATION.md, repo_truth, Decision Forum receipts |

## What reaches your desk

**Daily brief** — only items that:

1. Passed Decision Forum council routing
2. Received multi-provider AI-IRB crosschecks with receipts
3. Match presidential attention policy (Strategic/Constitutional, or Operational tagged `requires_human_ratification` / deadlock)

**Real-time push (Slack→SMS)** — only **CCIR / blocker** triggers (see [CCIR.md](CCIR.md), [CHAIRMAN-ESCALATION.md](CHAIRMAN-ESCALATION.md)).

Routine work never reaches the presidential desk.

## Hard rules

- `GAP-REGISTRY.md` is the only VCG execution ledger (referenced, never duplicated).
- No brief item without council route **and** receipt-bearing crosschecks.
- Heuristic ExoForge/Archon panel simulation is insufficient for presidential-bound items.
- Assurance theater forbidden (D9).
- Blockers always escalate to Executive Chairman in real time; push failure is receipted.
- Two-person gate: Bob (`bob-stewart`) + Max (`mstewartbz`) for irreversible/constitutional ratify.
- Permanent devil’s advocate seat on every presidential AI-IRB session.
- Dogfood manual IRB slice before automating Railway/Slack push.

## Artifact map

| File | Purpose |
|------|---------|
| [RUNTIME-TOPOLOGY.md](RUNTIME-TOPOLOGY.md) | Railway + GitHub lock |
| [MISSION-GRAPH.md](MISSION-GRAPH.md) / [mission-graph.yaml](mission-graph.yaml) | Ecosystem graph |
| [nodes/](nodes/) | Per-workstream drill-downs + Steer Packs |
| [AI-IRB-COHORT.md](AI-IRB-COHORT.md) | Provider seats |
| [DAILY-ATTENTION-PROTOCOL.md](DAILY-ATTENTION-PROTOCOL.md) | Brief orchestrator spec |
| [CHAIRMAN-ESCALATION.md](CHAIRMAN-ESCALATION.md) | Real-time blocker push |
| [CCIR.md](CCIR.md) | What may interrupt via Slack/SMS |
| [TWO-PERSON-GATE.md](TWO-PERSON-GATE.md) | Bob + Max dual attestation |
| [CROSSCHECK-RECEIPTS.md](CROSSCHECK-RECEIPTS.md) | Advisory ↔ receipt binding |
| [DOGFOOD.md](DOGFOOD.md) | Manual rehearsal before automation |
| [SECRETS-ACTIVATION.md](SECRETS-ACTIVATION.md) | Operator GitHub/Railway secret names + push gate |
| [CHAOS-DRILL.md](CHAOS-DRILL.md) | Monthly fail-closed drills |
| [STEER-PROTOCOL.md](STEER-PROTOCOL.md) | HOW-team steer packs |
| [PROGRESS.md](PROGRESS.md) | Rollup-by-reference |

## Trust boundary

These documents steer humans and agents. They do not mint consent, authority, provenance, or governance outcomes. Binding effects require Decision Forum / EXOCHAIN core APIs with verified receipts.
