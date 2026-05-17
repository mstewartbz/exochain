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

# Codex Security Findings Triage - 2026-05-16

Imported evidence:
`/Users/bobstewart/Downloads/codex-security-findings-2026-05-16T13-25-31.366Z.csv`

The CSV is external imported evidence and is not source-of-truth code. Each row
below is treated as an untrusted hypothesis until reproduced against current
`main`. Remediation must follow the EXOCHAIN core-first workflow: classify the
affected paths, write a failing regression or bounded source guard, make the
smallest owned enforcement change, run focused and relevant workspace gates,
then isolate the commit and PR from adjacent-surface work.

Current baseline when this triage was created:

- `main` fast-forwarded through `d0cd390a`.
- Source evidence contains 22 high-severity findings, all marked `new`.
- The raw CSV remains outside the repository.
- Pre-existing untracked `docs/heartfield/HEARTFIELD_AI_WHITEPAPER.md` remains
  unrelated to this triage.

## Priority Order

| Priority | Finding | Classification | Current status | First verification target |
|---|---|---|---|---|
| P0 | Session expiry uses deterministic HLC in production | Core runtime adapter: `crates/exo-gateway/src/server.rs`; EXOCHAIN core support: `crates/exo-core/src/hlc.rs` | Verified remediated on current main; no code change required | `cargo test -p exo-gateway production_app_state_uses_database_time_for_db_backed_session_expiry -- --nocapture`; `cargo test -p exo-gateway production_session_auth_rejects_epoch_expired_token -- --nocapture` |
| P0 | Bearer session TTL uses deterministic counter | Core runtime adapter: `crates/exo-node/src/zerodentity/api.rs`, `crates/exo-node/src/main.rs`; EXOCHAIN core support: `crates/exo-node/src/zerodentity/*`, `crates/exo-core/src/hlc.rs` | Verified remediated on current main; no code change required | `cargo test -p exo-node production_api_state -- --nocapture`; `cargo test -p exo-node store_session -- --nocapture` |
| P1 | Client-supplied authority accepted for settlements | EXOCHAIN core: `crates/exo-economy/src/settlement.rs`, `crates/exo-economy/src/value_contribution.rs`; core runtime adapter: `crates/exo-node/src/economy.rs` | Verified remediated on current main; no code change required | `cargo test -p exo-node automated_settlement_rejects_client_supplied_preconditions -- --nocapture`; `cargo test -p exo-economy automated_settlement_rejects_authority_proof_not_bound_to_adoption -- --nocapture` |
| P1 | Vote conflict checks trust caller-supplied affected DIDs | Core runtime adapter: `crates/exo-gateway/src/handlers.rs`; EXOCHAIN core: `crates/exo-governance/src/conflict.rs` | Verified remediated on current main; no code change required | `cargo test -p exo-gateway trusted_decision_affected_dids_block_conflict_even_when_request_context_is_unrelated -- --nocapture`; `cargo test -p exo-gateway vote_handler_derives_conflict_context_from_locked_decision_state -- --nocapture` |
| P1 | MCP trusts unsigned consent and override context | Core runtime adapter: `crates/exo-node/src/mcp/tools/authority.rs`, `crates/exo-node/src/mcp/middleware.rs` | Queued | Prove MCP authority tools reject unsigned or caller-fabricated consent and override context |
| P1 | Quorum counts unproven non-human votes as authentic | EXOCHAIN core: `crates/exo-gatekeeper/src/types.rs`, `crates/exo-gatekeeper/src/invariants.rs`; core runtime adapter: `crates/exochain-wasm/src/gatekeeper_bindings.rs` | Queued | Prove quorum vote provenance is verified for every counted voter class |
| P1 | AVC validation trusts caller approval flag | EXOCHAIN core: `crates/exo-avc/src/credential.rs`, `crates/exo-avc/src/validation.rs` | Queued | Prove human approval evidence is cryptographically bound, not a caller boolean |
| P1 | Single validator can mint arbitrary audit receipts | Core runtime adapter: `crates/exo-node/src/reactor.rs` | Queued | Prove audit receipts are bound to quorum, chain state, and trusted validator membership |
| P1 | Passport API reports active standing without verification | Core runtime adapter: `crates/exo-node/src/passport.rs`, `crates/exo-node/src/main.rs` | Queued | Prove passport active standing is registry-backed and not structurally inferred |
| P2 | WASM receipt verifier trusts caller-supplied actor keys | Core runtime adapter: `crates/exochain-wasm/src/catapult_bindings.rs`; EXOCHAIN core: `crates/exo-catapult/src/receipt.rs` | Queued | Prove WASM receipt verification cannot accept caller-minted DID key bindings |
| P2 | WASM governance trusts caller-supplied keys and roles | Core runtime adapter: `crates/exochain-wasm/src/decision_forum_bindings.rs`, `crates/exochain-wasm/src/governance_bindings.rs`; EXOCHAIN core: `crates/exo-governance/src/deliberation.rs` | Queued | Prove governance close/count paths use trusted key and role resolution |
| P2 | Bailment acceptance trusts caller-supplied bailee key | EXOCHAIN core: `crates/exo-consent/src/bailment.rs`, `crates/exo-consent/src/gatekeeper.rs`; core runtime adapter: `crates/exochain-wasm/src/consent_bindings.rs` | Queued | Prove bailee key is resolved from trusted DID state before acceptance |
| P2 | WASM authority verification skips chain topology validation | Core runtime adapter: `crates/exochain-wasm/src/authority_bindings.rs`; EXOCHAIN core: `crates/exo-authority/src/chain.rs` | Queued | Prove WASM authority-chain verification rejects broken topology and caller-only key maps |
| P2 | WASM decision transitions can disable all invariants | Core runtime adapter: `crates/exochain-wasm/src/decision_forum_bindings.rs`, `packages/exochain-wasm/test/bridge_verification.mjs`; EXOCHAIN core: `crates/exo-gatekeeper/src/invariants.rs` | Verified remediated on current main; no code change required | `cargo test -p exochain-wasm wasm_decision_transition_requires_kernel_adjudication -- --nocapture`; `node packages/exochain-wasm/test/bridge_verification.mjs` |
| P2 | Governance attestations trust caller-supplied keys | Adjacent surface: `demo/services/audit-api/src/index.js`; core runtime adapter: `crates/exochain-wasm/src/gatekeeper_bindings.rs` | Queued behind core-owned runtime issues | Prove governance health attestation keys are pinned or registry-resolved before persistence |
| P2 | Plaintext hashes leak encrypted message contents | EXOCHAIN core: `crates/exo-messaging/src/envelope.rs`, `crates/exo-messaging/src/compose.rs`, `crates/exo-messaging/src/open.rs` | Queued | Prove encrypted-envelope metadata cannot be used as a plaintext equality oracle |
| P2 | Identity erasure deletes third-party conflict records | Core runtime adapter: `crates/exo-gateway/src/server.rs`, `crates/exo-gateway/src/db.rs`, `crates/exo-gateway/src/handlers.rs` | Queued | Prove identity erasure tombstones subject-owned data without deleting independent conflict records |
| P2 | Unbounded DB DID registration enables storage DoS | Core runtime adapter: `crates/exo-gateway/src/server.rs`, `crates/exo-gateway/src/db.rs`, `crates/exo-gateway/migrations/20260504000003_create_did_documents.sql`; EXOCHAIN core: `crates/exo-identity/src/registry.rs` | Queued | Prove tenant-scoped DID registration has durable quota or admission control |
| P2 | Gateway rate limit collapses clients behind proxies | Core runtime adapter: `crates/exo-gateway/src/server.rs` | Queued | Prove rate limiting uses a trusted deployment identity and avoids spoofed forwarded headers |
| P2 | Conflict recusal checks are capped at 1000 declarations | Core runtime adapter: `crates/exo-gateway/src/db.rs`, `crates/exo-gateway/src/server.rs`, `crates/exo-gateway/src/handlers.rs`; EXOCHAIN core: `crates/exo-governance/src/conflict.rs` | Queued | Prove conflict checks are complete or fail closed when the declaration set exceeds bounded read limits |
| P2 | P2P rate limiter slot cap can be permanently exhausted | EXOCHAIN core: `crates/exo-api/src/p2p.rs`; core runtime adapter: `crates/exo-node/src/network.rs` | Queued | Prove stale or abusive P2P limiter slots are evicted deterministically |
| P3 | Untrusted ExoForge issues can drive unapproved code changes | Adjacent workflow surface: `archon/workflows/*`, `archon/commands/*` | Queued after core/runtime issues | Prove issue/workflow prose is bounded as untrusted input before authorizing code, GitHub, or merge operations |

## Tracking Notes

- Findings that mention `demo/`, `site/`, `archon/`, or other non-Rust product
  shells remain adjacent unless a tested runtime adapter proves they can read,
  write, or assert EXOCHAIN core decisions.
- WASM findings are classified as core runtime adapter issues when the export
  can carry consent, authority, provenance, governance, or settlement decisions
  across an untrusted caller boundary.
- Session expiry findings are first because they can invalidate authentication
  and owner-only identity access controls across live runtime adapters.

## Verification Log

### P0 - Session Expiry Uses Deterministic HLC In Production

Disposition on 2026-05-17: verified already remediated on current `main`.

Path classification:

- Core runtime adapter: `crates/exo-gateway/src/server.rs`.
- EXOCHAIN core support: `crates/exo-core/src/hlc.rs`.
- Imported evidence tracking: this file.

Current enforcement evidence:

- Production `AppState::new` constructs DB-backed gateway state with
  `SessionTimeSource::DatabaseWhenAvailable`.
- `trusted_session_validation_time_ms` uses `trusted_database_epoch_ms` when a
  DB pool is present, falling back to HLC only for injected-clock/dev paths.
- `require_authenticated_session_actor_for_token` queries
  `sessions` with `expires_at > trusted_session_validation_time_ms(state)`.
- Session validation source guards prove caller-supplied observed-at headers do
  not drive DB-backed session expiry checks.

Validation commands:

```bash
cargo test -p exo-gateway production_app_state_uses_database_time_for_db_backed_session_expiry -- --nocapture
cargo test -p exo-gateway session_validation_uses_gateway_clock_not_caller_header_time -- --nocapture
cargo test -p exo-gateway production_session_auth_rejects_epoch_expired_token -- --nocapture
```

### P0 - Bearer Session TTL Uses Deterministic Counter

Disposition on 2026-05-17: verified already remediated on current `main`.

Path classification:

- Core runtime adapter: `crates/exo-node/src/zerodentity/api.rs` and
  `crates/exo-node/src/main.rs`.
- EXOCHAIN core support: `crates/exo-node/src/zerodentity/*` and
  `crates/exo-core/src/hlc.rs`.
- Imported evidence tracking: this file.

Current enforcement evidence:

- Production `ApiState::new` does not install `HybridClock::new()` or any
  deterministic session clock.
- Production 0dentity session-protected routes fail closed with
  `Trusted 0dentity session clock unavailable` unless a trusted clock is
  explicitly injected by `new_with_clock`.
- `ZerodentityStore::get_session` rejects revoked sessions, expired sessions,
  future-created sessions, and expiry-deadline arithmetic overflow.
- API-level session reads reject retained bearer tokens once the absolute
  24-hour `IDENTITY_SESSION_TTL_MS` deadline is reached.

Validation commands:

```bash
cargo test -p exo-node production_api_state -- --nocapture
cargo test -p exo-node list_claims_rejects_expired_session -- --nocapture
cargo test -p exo-node store_session -- --nocapture
```

### P1 - Client-Supplied Authority Accepted For Settlements

Disposition on 2026-05-17: verified already remediated on current `main`.

Path classification:

- EXOCHAIN core: `crates/exo-economy/src/settlement.rs` and
  `crates/exo-economy/src/value_contribution.rs`.
- Core runtime adapter: `crates/exo-node/src/economy.rs`.
- Imported evidence tracking: this file.

Current enforcement evidence:

- The node economy API rejects client-supplied automated-settlement
  preconditions instead of trusting request-provided `authority_valid` flags.
- Runtime automated settlement derives preconditions from stored offer,
  acceptance, adoption, contribution-node, wrapper, ruleset, and value-event
  records.
- Request `automation_authority_ref` must match the stored contribution
  acceptance authority envelope and adoption proof hash.
- Core settlement preconditions fail closed when delegated authority is invalid
  or a dispute is active.

Validation commands:

```bash
cargo test -p exo-node automated_settlement_rejects_client_supplied_preconditions -- --nocapture
cargo test -p exo-node automated_settlement_rejects_request_authority_not_bound_to_stored_acceptance -- --nocapture
cargo test -p exo-economy automated_settlement_rejects_authority_proof_not_bound_to_adoption -- --nocapture
cargo test -p exo-economy automated_preconditions_fail_closed_for_missing_authority_and_active_dispute -- --nocapture
```

### P1 - Vote Conflict Checks Trust Caller-Supplied Affected DIDs

Disposition on 2026-05-17: verified already remediated on current `main`.

Path classification:

- Core runtime adapter: `crates/exo-gateway/src/handlers.rs`.
- EXOCHAIN core: `crates/exo-governance/src/conflict.rs`.
- Imported evidence tracking: this file.

Current enforcement evidence:

- The vote handler loads the stored `DecisionObject` under a row lock and
  derives conflict `affected_dids` from the decision's trusted metadata.
- Request-body `affected_dids` can no longer make conflict adjudication
  unrelated or empty.
- Conflict recusal uses `load_blocking_conflict_declarations_for_vote` with
  trusted affected DIDs and fails closed when the trusted context is empty.
- `check_and_block` remains the enforcing gate before kernel adjudication and
  vote persistence.

Validation commands:

```bash
cargo test -p exo-gateway trusted_decision_affected_dids_block_conflict_even_when_request_context_is_unrelated -- --nocapture
cargo test -p exo-gateway vote_handler_derives_conflict_context_from_locked_decision_state -- --nocapture
cargo test -p exo-gateway vote_handler_source_does_not_default_conflict_adjudication -- --nocapture
cargo test -p exo-gateway conflict_declaration_loader_rejects_empty_recusal_context -- --nocapture
```

### P2 - WASM Decision Transitions Can Disable All Invariants

Disposition on 2026-05-17: verified already remediated on current `main`.

Path classification:

- Core runtime adapter: `crates/exochain-wasm/src/decision_forum_bindings.rs`
  and `packages/exochain-wasm/test/bridge_verification.mjs`.
- EXOCHAIN core: `crates/exo-gatekeeper/src/invariants.rs` and
  `crates/decision-forum/src/decision_object.rs`.
- Imported evidence tracking: this file.

Current enforcement evidence:

- `wasm_transition_decision` fails closed with
  `unadjudicated decision transitions are disabled`.
- `wasm_transition_decision_adjudicated` rejects caller-supplied
  `invariant_set`.
- The adjudicated WASM path constructs the kernel with `InvariantSet::all()`.
- `DecisionObject::transition_at` rejects raw BCTS transition attempts, and
  `transition_adjudicated_at` mutates state only after `Kernel::adjudicate`
  returns `Verdict::Permitted`.

Validation commands:

```bash
cargo test -p exochain-wasm wasm_decision_transition_requires_kernel_adjudication -- --nocapture
node packages/exochain-wasm/test/bridge_verification.mjs
```
