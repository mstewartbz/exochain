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

# Live Dogfood Item — Presidential C2 push enablement

**Item class:** Constitutional / irreversible (requires Bob + Max dual gate)  
**Mission node:** `platform-release` (see `docs/c2/nodes/platform-release.md`)  
**Status:** Awaiting human dual attestation  
**Machine bridge:** already green via `presidential_c2_bridge`

## Decision under review

Enable live Slack→SMS CCIR push for Mission C2 presidential attention **only after**:

1. PR #791 is on `main`
2. GitHub + Railway secrets from `docs/c2/SECRETS-ACTIVATION.md` are set by operators
3. This item is dual-ratified below

## AI-IRB rehearsal (manual / role-differentiated)

Operators paste provider advisories + dissent receipt IDs here (do not treat chat as authority):

| Seat | Role | Advisory hash / receipt | Dissent? |
|------|------|-------------------------|----------|
| xAI / Grok | Strategic | _pending_ | _pending_ |
| OpenAI | Skeptic | _pending_ | _pending_ |
| Anthropic | Devil’s advocate (mandatory) | _pending_ | **required** |

## Dual gate (binding)

Principals (immutable):

- Bob: `did:exo:principal:bob-stewart` / GitHub `bob-stewart`
- Max: `did:exo:principal:mstewartbz` / GitHub `mstewartbz`

| Principal | Vote | Timestamp (HLC or wall for human log) | Signature / GH attestation |
|-----------|------|----------------------------------------|----------------------------|
| Bob | Approve / Veto | | |
| Max | Approve / Veto | | |

**Rule:** both must Approve for ratify. Either Veto blocks push enablement.

## CCIR calibration notes from this item

| Would interrupt via Slack/SMS? | Why |
|--------------------------------|-----|
| Yes — missing dual gate before push | Constitutional |
| No — routine CI queue delay | Brief-only noise |

## Closeout

When both Approve rows are filled, update
`docs/c2/DOGFOOD-REHEARSAL-2026-07-12.md` status to **Complete**, then enable
push emission (still fail-closed if secrets absent).
