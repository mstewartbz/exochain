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

# HonorGood Compact

Status: canonical doctrine for EXOCHAIN economy primitives.

## Compact

We built. We use. We honor good.

HonorGood records useful contribution dignity, evergreen provenance, upstream open-source recognition, and conditional value participation when downstream value materially derives from recorded upstream work.

HonorGood is not a payment rail, equity grant, legal conclusion, or marketing claim. It is an EXOCHAIN-native provenance and settlement primitive. Economic effect requires recorded terms, acceptance or ratification where required, deterministic rulesets, valid value events, and settlement records.

## Doctrine

Mission creates context.
Purpose creates alignment.
Receipts create proof.
Rulesets create fairness.
Settlement creates trust.
EXOCHAIN records the whole thing.

Membership creates access.
Contribution creates receipts.
Receipts create settlement.
EXOCHAIN creates trust.

Humans govern the covenant.
Holons execute the covenant.
EXOCHAIN records the covenant.
Settlement follows the covenant.

## Core Objects

HonorGood is represented in `crates/exo-economy` by:

- `ValueContributionNode`: the offered contribution node.
- `ContributionOffer`: the recorded offer of terms.
- `ContributionAcceptance`: accepted terms under authority.
- `BailmentTerms` and `BailmentWrapper`: custody and use wrapper.
- `AdoptionEvent`, `UseEvent`, and `ValueEvent`: adoption, use, and value proof.
- `ContributionReceipt`: mission or contribution workflow proof.
- `LegacyReceipt`: upstream provenance and conditional participation state.
- `HonorGoodRuleset`: deterministic share and review policy.
- `MissionSettlement` and `AutomatedSettlementEvent`: checked settlement accounting.

EXOCHAIN core is the settlement authority. CommandBase can be a cockpit. ExoForge can be a factory. Neither becomes the source of settlement truth.

## Required Guardrails

- Deterministic canonical CBOR hashing with BLAKE3 domain tags.
- Integer-only arithmetic and basis points.
- Checked settlement arithmetic that fails closed on overflow or underflow.
- Per-basis allocation totals not above 10,000 basis points.
- HLC timestamps only.
- Opaque beneficiaries only: DIDs, hashes, vault pointers, or public project treasury references.
- No sensitive personal, banking, tax, family, estate, or payment data on-ledger.
- No payment, equity, legal obligation, or ownership transfer unless recorded terms and legal effect support it.
- No automated settlement from unaccepted offers, unratified upstream claims, revoked nodes, suspended nodes, disputed materiality, or missing authority.
- No trust claim by proximity from adjacent surfaces.

## Legal Effect

Legacy and contribution terms use explicit legal-effect states:

- `VoluntaryRecognitionOnly`
- `OfferedTerms`
- `AcceptedTerms`
- `ContributorAccepted`
- `RatifiedAgreement`
- `Revoked`
- `Superseded`

Archon and Paperclip seed receipts are unratified. They remain proposed recognition records unless signed contributor acceptance and human ratification are recorded.
