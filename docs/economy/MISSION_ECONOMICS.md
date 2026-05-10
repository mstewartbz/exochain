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

# Mission Economics

Mission economics are the EXOCHAIN core accounting model for purpose-bound work.

## Core Pattern

- `Mission` defines the economic container.
- `MissionPurpose` defines the problem, served party, promised outcome, risk surface, proof, and success condition.
- `ContributionReceipt` records useful work inside a Mission or contribution workflow.
- `HonorGoodRuleset` defines share lines per settlement basis.
- `MissionSettlement` computes settlement lines with checked integer arithmetic.

Settlement authority remains in EXOCHAIN core. CommandBase can show Mission state. ExoForge can propose receipts or rulesets. Neither simulates authoritative settlement locally.

## Runtime Authority

`exo-node` exposes the core Mission Economics routes under
`/api/v1/economy/*`. The route layer verifies required stored
predecessors before recording dependent objects:

- contribution receipts require a stored mission or contribution node when those IDs are present;
- contribution acceptances require a stored offer and matching accepted terms;
- bailment wrappers require stored terms, offer, acceptance, and authority references;
- adoption, use, and value events require their recorded predecessor chain;
- mission settlements require a stored mission and stored ruleset;
- automated settlements require stored node, adoption, use, value event, wrapper, ruleset, valid authority, sufficient legal effect, and fail-closed preconditions.

Accepted objects are stored as canonical CBOR in the node database and appended
to the `EconomyRecordAnchor` hash chain. The chain records object kind, ID,
content hash, HLC timestamp, and previous anchor hash.

## Accounting Rules

- No floats.
- Basis points only for fractional allocation.
- Each settlement basis is validated independently.
- Basis totals must not exceed 10,000 basis points.
- Overflow and underflow fail closed.
- Unsupported basis values fail closed.
- Zero amounts require explicit `ZeroFeeReason`.
- Payment, fiat, token, exchange, and external custody rails are outside this core accounting path.

## Adjacent Adapters

CommandBase is the cockpit. Its HonorGood routes proxy requests to the EXOCHAIN
economy API and return `local_simulation: false` on adapter errors.

ExoForge is the factory. Its HonorGood command can generate unratified legacy
receipt proposals and submit complete core payloads to EXOCHAIN, but EXOCHAIN
core validates, hashes, anchors, and settles.

The WASM bridge exposes deterministic validation and anchor helpers for stable
Mission, LegacyReceipt, HonorGoodRuleset, and ValueContributionNode payloads. It
does not add payment execution or local settlement authority.

## Apex Velocity Catalyst

Use explicit `ApexVelocityCatalyst` naming in code and docs where ambiguity exists. Bare `AVC` remains Autonomous Volition Credential.
