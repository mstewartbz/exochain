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


# Executive Chairman Escalation

**Seat:** Executive Chairman — Bob Stewart (`bob-stewart`).  
**Co-principal on irreversible acts:** Max Stewart (`mstewartbz`) per [TWO-PERSON-GATE.md](TWO-PERSON-GATE.md).

## Triggers (blockers)

- Council / AI-IRB deadlock or quorum failure after max rounds
- Same validation failure fingerprint twice (AGENTS.md)
- Mission Graph node kill-criteria tripwire
- Human-gate item past SLA with no ratify/veto
- Security/Governance veto stopping a critical path
- Prior escalation push/receipt integrity failure
- Chaos-drill failure (see [CHAOS-DRILL.md](CHAOS-DRILL.md))

## Delivery precedence

1. **Slack** (primary) — CommandBase `sendSlackMessage` webhook; deep-link to Presidential Desk dossier
2. **SMS** (Twilio) — if Slack fails or severity ≥ `critical`
3. **In-app** — always write `decision_needed` / escalation notification even if push fails

## Receipt

Every push emits: `escalation_id`, trigger class, decision/node refs, channels attempted, delivery status per channel, fingerprint, timestamp (HLC in core; hash chain adjacent).

Dedupe by `escalation_fingerprint`; coalesce related blockers. Non-CCIR must **not** push ([CCIR.md](CCIR.md)).

## Inbound

Slack may **ack / inquire**. Ratify/veto require authenticated desk or signed action — never emoji-only approval.

## Unreachable

Both channels fail → `chairman_unreachable` fault on next daily brief.
