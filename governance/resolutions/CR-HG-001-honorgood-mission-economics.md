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

# CR-HG-001: HonorGood And Mission Economics

Status: draft resolution for core economy extension and runtime adapters.

## Resolution

EXOCHAIN recognizes HonorGood and Apex Velocity Catalyst Mission Economics as deterministic economy and provenance primitives implemented in `exo-economy`.

This resolution does not add a kernel invariant. `ProvenanceHonoring` is documented as a governance concept only.

## Determinism

- Canonical CBOR hashing with versioned BLAKE3 domains.
- HLC timestamps only.
- Integer-only `MicroExo` accounting.
- Basis points for fractional shares.
- Checked arithmetic for core settlement.
- Ordered collections only.

## Safety

- No payment gating for trust, identity, consent, Autonomous Volition Credential validation, or governance access.
- No fiat rails, token rails, external exchanges, or automated payments.
- Opaque beneficiary references only.
- No sensitive personal, banking, tax, family, estate, or payment data on-ledger.
- No adjacent-surface trust claim by proximity.
- No implied legal obligation for unaccepted upstream recognition.
- Human approval required for ratification, legal-template changes, disputed materiality, revocation, off-policy use, and high-risk custody.

## Core Objects

The core object set includes Mission, MissionPurpose, ContributionReceipt, LegacyReceipt, ValueContributionNode, ContributionOffer, ContributionAcceptance, BailmentTerms, BailmentWrapper, AdoptionEvent, UseEvent, ValueEvent, HonorGoodRuleset, SettlementLine, MissionSettlement, and AutomatedSettlementEvent.

## Runtime Path

`exo-node` records HonorGood and Mission Economics objects through
`/api/v1/economy/*` routes. The route layer requires stored predecessor objects
for accepted terms, bailment wrappers, adoption, use, value, mission settlement,
and automated settlement. Accepted objects are stored as canonical CBOR in the
node database and hash-linked with `EconomyRecordAnchor`.

CommandBase is the cockpit adapter. ExoForge is the factory adapter. Both are
adjacent surfaces with intake records. They can submit to or display EXOCHAIN
core responses, but they do not become sources of settlement truth.

The WASM bridge exposes stable validation and anchor helpers for Mission,
LegacyReceipt, HonorGoodRuleset, and ValueContributionNode payloads only.

## Threats Addressed

- provenance tampering by content hashes;
- replay and ordering ambiguity by HLC and hash links;
- overallocated settlement by per-basis basis-point validation;
- overflow and underflow by checked arithmetic;
- unratified upstream claims by LegacyReceipt state machine;
- adjacent-surface authority confusion by core-only settlement authority;
- sensitive beneficiary exposure by opaque references.

## Non-Claims

The resolution does not establish token economics, payment execution, equity, legal advice, ownership transfer, or automatic upstream legal obligation.
