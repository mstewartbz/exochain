//! ExoChain WASM Bindings
//!
//! Exposes the full ExoChain governance engine to JavaScript/Node.js via WebAssembly.
//! Covers 13 crates: core, identity, consent, authority, gatekeeper, governance,
//! escalation, legal, dag, proofs, api, tenant, and decision-forum.

mod authority_bindings;
mod consent_bindings;
mod core_bindings;
mod decision_forum_bindings;
mod escalation_bindings;
mod gatekeeper_bindings;
mod governance_bindings;
mod identity_bindings;
mod legal_bindings;
mod serde_bridge;
