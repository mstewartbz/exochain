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


# Two-Person / Two-System Gate

## Principals (locked)

| Principal | GitHub | Role |
|-----------|--------|------|
| Bob Stewart | `bob-stewart` | Executive Chairman / CTO — first attestation |
| Max Stewart | `mstewartbz` | Co-principal — independent second attestation |

Canonical DIDs (runtime):

- `did:exo:principal:bob-stewart`
- `did:exo:principal:mstewartbz`

## Applies to

- Constitutional-class decisions
- Irreversible ratify/veto
- Emergency override ratification
- Actions that mint or revoke trust claims

## Rules

- Both attestations separately authenticated; recorded on decision receipt
- Agents, AI-IRB seats, Slack acks, CommandBase personas **cannot** satisfy either half
- Either principal may **veto**; irreversible **ratify** requires **both** Approve votes from verified DIDs
- Operational (policy-tagged) may allow single-principal ratify; when in doubt, dual gate
- Same browser/session/agent cannot attest as both

## Core API

`decision_forum::human_gate::enforce_two_person_ratification`  
`decision_forum::human_gate::two_person_veto_present`

## Failure modes

| Case | Result |
|------|--------|
| Only Bob attested | `TwoPersonGateRequired` |
| Only Max attested | `TwoPersonGateRequired` |
| Agent DID in verified set attempting half | Rejected (not a principal) |
| Either Reject vote | Veto present; no ratification |
