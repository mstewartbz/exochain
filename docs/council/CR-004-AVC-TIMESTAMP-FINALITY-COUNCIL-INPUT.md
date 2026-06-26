---
title: "Council Input for CR-004: AVC Timestamp and Finality Authority"
status: recorded
created: 2026-06-21
tags: [council, avc, timestamp, finality, evidence]
links:
  - "[[CR-004-AVC-TIMESTAMP-FINALITY-AUTHORITY]]"
---
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

# Council Input for CR-004: AVC Timestamp and Finality Authority

**Input ID:** CR-004-COUNCIL-INPUT
**Review time:** 2026-06-21T15:06:39.919Z
**Method:** ExoForge 5-panel council-style heuristic triage
**Binding:** No. This is advisory council input, not a substitute for human member votes.
**Chair directive:** Bob Stewart, acting as Chair, directed completion absent a member veto; any member veto pauses activation and triggers Chair intervention.

---

## Proposal Reviewed

Council input was requested for a Civilizational-Class AVC timestamp and finality authority model.

The reviewed proposal selects a layered evidence standard:

1. Validator-signed AVC receipts provide internal EXOCHAIN protocol evidence.
2. EXOCHAIN DAG, BCTS, or equivalent governed finality commitment provides tamper-evident EXOCHAIN ordering and adjudication evidence.
3. Independent external timestamp or anchoring evidence provides the additional time authority required for civilizational, court-grade, or public reliance claims.

The proposal rejects fabricated external timestamp claims, production `SystemTime::now` shortcuts, requester self-approval, and any production-operational claim before authority material, verification tests, deployment evidence, and rollback evidence are present.

---

## Panel Results

| Panel | Vote | Confidence | Findings | Conditions |
|-------|------|------------|----------|------------|
| Governance | Approve with conditions | 0.60 | Authority-chain or delegation changes detected. | Human gate verification is required for the decision class. |
| Legal | Approve | 0.60 | Evidence-handling implications. | Evidence chain review is required and non-blocking. |
| Architecture | Approve | 0.60 | BCTS state transitions may be affected. | State-transition review is recommended. |
| Security | Approve with conditions | 0.60 | Cryptographic operations affected. | Authority key material and signing paths require security audit. |
| Operations | Approve | 0.60 | Deployment path changes detected. | Deployment plan and rollback evidence must be documented. |

**Tally verdict:** APPROVED
**Score:** 0.465
**Vetoed by:** none
**Panels reviewed:** 5
**Total findings:** 5

---

## Council Input Summary

The council-style review supports adopting the layered AVC evidence model with conditions. No panel veto was recorded. Governance and Security conditions are treated as activation gates: human-gate verification and cryptographic authority review must complete before any production civilizational-class claim is made.

The input supports immediate resolution of the model question, while reserving production activation for evidence that the selected authority material is configured, independently verifiable, and fail-closed.
