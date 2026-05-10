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

# Automated Value-For-Value Settlement

`AutomatedSettlementEvent` records routine deterministic settlement when pre-approved terms and authority are already in place.

## Required Conditions

Automated settlement may execute only when:

- the contribution was offered under pre-approved terms;
- the adopter is authorized;
- the adopting agent or holon acts inside a delegated authority envelope;
- the ruleset is deterministic and active;
- the value event is measurable and valid;
- the settlement basis is supported;
- required signatures, receipts, and proofs are present;
- no dispute, revocation, suspension, high-risk custody exception, materiality dispute, or constitutional conflict is active;
- legal effect permits settlement.

## Human Approval

Human approval is required for new or changed legal templates, disputed materiality, unratified upstream claims, off-policy use, high-risk custody, settlement-term changes, revocation, constitutional conflicts, and exceptions to approved rulesets.

## Zero Launch

Automated settlement can record zero amounts. A zero result must carry an explicit reason. No automatic fiat, token, exchange, or payment execution is implemented by this primitive.
