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

# fix-onyx-4-r5-holons-stub-context

This is a minimal pointer document, not an implementation plan. This
initiative is tracked as **GAP-REGISTRY.md row VCG-010** and is governed
by **ratified decision D5** (root legitimacy: witnessed ceremony plus
external attestation plus lineage; self-issued roots are rejected;
Scaling-Holon promotion is recommendation-only).

## Current state (as of the VCG-010 remediation)

`crates/exo-node/src/holons.rs` — infrastructure Holon adjudication
contexts carry real Ed25519 authority and provenance signatures
(`build_holon_adjudication_context`, `signed_authority_link`,
`signed_provenance`). The stale "sentinel `vec![1, 2, 3]`" signatures this
initiative originally tracked no longer exist.

Two residual gaps were closed under this initiative:

1. **Self-issued root rejection.** `build_holon_adjudication_context` no
   longer trusts `config.root_did`'s key merely because it is the key that
   signed the root→Holon authority link. Trust for the root authority is
   populated only from an independent `RootAttestation` that is distinct
   from the signer on **both** axes D5 requires: a different `attester_did`
   label **and** a different `attester_public_key` from
   `config.root_public_key`. An attestation carrying a distinct DID label
   but reusing the root's own key (self-issuance laundered through a second
   label) fails the key-inequality guard and falls through to the
   self-issued arm. With no `root_attestation` configured, or a
   key-reusing one, the kernel's `AuthorityChainValid` invariant fails
   closed and `holon::step` rejects the root.
   Remaining D5 depth (a fully witnessed multi-party ceremony and wiring an
   external attestation into the production default, which currently sets
   `root_attestation: None` and is therefore fail-closed) is tracked as
   follow-on work, not claimed complete by this change.
2. **Scaling Holon recommendation-only, full stop.** The Scaling Holon's
   auto-promotion path no longer calls `reactor::submit_proposal`. It
   emits a `GovernanceEventType::RecommendationOnly` event carrying the
   same named candidate/evidence a real `ValidatorSetChange::AddValidator`
   proposal would have carried, and submits zero DAG proposals. Promotion
   remains a ratification event with named evidence, never an automatic
   state change.

## Why the runtime still refuses by default

No external-attestation source (a witnessed ceremony distinct from the
node's own signer) is wired into production yet — see
`crates/exo-node/src/main.rs`, the `unaudited-infrastructure-holons` block,
where `root_attestation` is set to `None`. Until a real external attester
(for example an `exo-authority` DelegationRegistry entry per ratified
decision D3, or an equivalent out-of-band witnessed-ceremony record) is
wired in, every root authority built from that config is self-issued, and
the kernel correctly rejects every infrastructure Holon step.

This tool/runtime stays refusing (compiled out of the default build) unless
the crate is built with the `unaudited-infrastructure-holons` feature (see
`crates/exo-node/Cargo.toml`), which is never safe to enable in production
until a real external-attestation source exists. See GAP-REGISTRY.md row
VCG-010 for scope and status.
