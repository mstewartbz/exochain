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

# fix-mcp-legal-simulation-tools

This is a minimal pointer document, not an implementation plan. This
initiative is tracked as **GAP-REGISTRY.md row VCG-004**.

## Refusal payload that cites this file

`crates/exo-node/src/mcp/tools/legal.rs` — the shared refusal helper
used by `exochain_ediscovery_search`, `exochain_assert_privilege`,
`exochain_initiate_safe_harbor`, and `exochain_check_fiduciary_duty`.
Its refusal payload's `initiative` field cites
`Initiatives/fix-mcp-legal-simulation-tools.md`:

> "This MCP legal tool has no live legal/evidence runtime attached, so
> it cannot search evidence, assert privilege, initiate safe harbor,
> or assess fiduciary duty."

These tools refuse by default and stay refusing unless the crate is
built with the `unaudited-mcp-simulation-tools` feature (see
`crates/exo-node/Cargo.toml`), which is never safe to enable in
production. See GAP-REGISTRY.md row VCG-004 for scope and status.
