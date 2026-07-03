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

# fix-mcp-simulation-tools

This is a minimal pointer document, not an implementation plan. This
initiative is tracked as **GAP-REGISTRY.md row VCG-004**.

## Refusal payloads that cite this file

Several MCP tool families would otherwise return truthy-shaped
responses without a live backing store or reactor wiring. Their
refusal payloads' `initiative` field cites
`Initiatives/fix-mcp-simulation-tools.md`:

- `crates/exo-node/src/mcp/tools/governance.rs` — decision/vote/amendment
  mutation tools.
- `crates/exo-node/src/mcp/tools/ledger.rs` — `exochain_submit_event`.
- `crates/exo-node/src/mcp/tools/escalation.rs` — case escalation/triage/
  feedback mutation tools.
- `crates/exo-node/src/mcp/tools/identity.rs` — identity creation/
  resolution/risk/passport mutation tools.

These tools refuse by default and stay refusing unless the crate is
built with the `unaudited-mcp-simulation-tools` feature (see
`crates/exo-node/Cargo.toml`), which is never safe to enable in
production. See GAP-REGISTRY.md row VCG-004 for scope and status.
