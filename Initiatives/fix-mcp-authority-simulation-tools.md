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

# fix-mcp-authority-simulation-tools

This is a minimal pointer document, not an implementation plan. This
initiative is tracked as **GAP-REGISTRY.md row VCG-004**.

## Refusal payload that cites this file

`crates/exo-node/src/mcp/tools/authority.rs`, `authority_tool_refused`
— used by `exochain_delegate_authority`. Its refusal payload's
`initiative` field cites
`Initiatives/fix-mcp-authority-simulation-tools.md`:

> "This MCP authority tool would otherwise return a simulation success
> without a signed authority store write or caller-supplied verified
> context. It is disabled in every build until it is wired to signed
> authority-store persistence."

Note: `exochain_verify_authority_chain`, `exochain_check_permission`,
and `exochain_adjudicate_action` in the same file are live read-only
tools — they are not gated by this initiative or by
`unaudited-mcp-simulation-tools`. Only `exochain_delegate_authority`
(a mutation) is in scope here.

This tool refuses by default and stays refusing unless the crate is
built with the `unaudited-mcp-simulation-tools` feature (see
`crates/exo-node/Cargo.toml`), which is never safe to enable in
production. See GAP-REGISTRY.md row VCG-004 for scope and status.
