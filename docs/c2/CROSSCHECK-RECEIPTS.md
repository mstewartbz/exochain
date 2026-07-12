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


# Crosscheck Advisories and Receipts

## Rule

No presidential brief item without:

1. Decision Forum council route evidence
2. Per-seat AI-IRB advisories (active providers)
3. At least one dissent/devil’s advocate receipt object (mandatory)
4. Bound custody / lifecycle receipt IDs

Missing any → **fail closed** (item stays off brief).

## Binding schema

See [`schemas/attention-item.schema.json`](schemas/attention-item.schema.json).

Fields: `decision_id`, `council_receipt_id`, `advisory_receipts[]` (provider, model_id, role, response_hash), `dissent_receipt_id`, `crosscheck_aggregate_hash`.

## Promotion path

Reuse Decision Forum `LifecycleReceipt` + CrossChecked custody patterns; do not treat demo CrossChecked API as SSOT. Adapter must verify hashes.
