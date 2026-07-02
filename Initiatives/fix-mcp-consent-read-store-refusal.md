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

# fix-mcp-consent-read-store-refusal

This is a minimal pointer document, not an implementation plan. This
initiative is tracked as **GAP-REGISTRY.md row VCG-004**.

## Refusal payloads that cite this file

`crates/exo-node/src/mcp/tools/consent.rs`:

- `consent_registry_unavailable` — used by `exochain_check_consent`
  and `exochain_list_bailments`. Refuses because there is no live
  consent registry attached to prove active consent or enumerate
  bailments.
- `consent_store_unavailable` — used by `exochain_propose_bailment`
  and `exochain_terminate_bailment`. Refuses because there is no live
  signed consent store attached to create or terminate bailments.

Both refusal payloads' `initiative` field cites
`Initiatives/fix-mcp-consent-read-store-refusal.md`.

These tools refuse by default and stay refusing unless the crate is
built with the `unaudited-mcp-simulation-tools` feature (see
`crates/exo-node/Cargo.toml`), which is never safe to enable in
production. See GAP-REGISTRY.md row VCG-004 for scope and status.
