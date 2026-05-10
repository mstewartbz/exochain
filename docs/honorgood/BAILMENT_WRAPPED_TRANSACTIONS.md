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

# Bailment-Wrapped Transactions

A bailment-wrapped transaction links contribution adoption, use, value measurement, and settlement to accepted terms.

## Flow

1. A `ValueContributionNode` is offered.
2. A `ContributionOffer` binds terms, permitted use, prohibited use, adoption policy, and settlement ruleset.
3. A `ContributionAcceptance` records accepted terms and delegated authority.
4. A `BailmentWrapper` binds the contribution, offer, acceptance, terms, custody scope, and authority references.
5. `AdoptionEvent`, `UseEvent`, and `ValueEvent` establish use and measurable value.
6. `AutomatedSettlementEvent` may execute only if all fail-closed checks pass.

## Fail-Closed Conditions

Automated settlement is rejected if any required offer, acceptance, wrapper, authority proof, ruleset, value event, legal effect, or materiality proof is missing or invalid. It is also rejected when a dispute, revocation, suspension, high-risk custody exception, or human-approval requirement is active.
