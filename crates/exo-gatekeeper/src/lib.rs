//! EXOCHAIN Gatekeeper — the Judicial Branch.
//!
//! This crate implements the Constitutional Governance Runtime (CGR):
//! - **Kernel** — immutable adjudicator enforcing constitutional invariants
//! - **Invariants** — the eight constitutional invariants
//! - **Combinator** — deterministic algebra for composing governance operations
//! - **Holon** — autonomous agent runtime with kernel-adjudicated steps
//! - **MCP** — Model Context Protocol enforcement for AI systems
//! - **TEE** — Trusted Execution Environment attestation

pub mod combinator;
pub mod error;
pub mod holon;
pub mod invariants;
pub mod kernel;
pub mod mcp;
pub mod mcp_audit;
pub mod tee;
pub mod types;

// Re-export primary types.
pub use combinator::{Combinator, CombinatorInput, CombinatorOutput};
pub use error::GatekeeperError;
pub use holon::{Holon, HolonState};
pub use invariants::{ConstitutionalInvariant, InvariantEngine, InvariantSet};
pub use kernel::{ActionRequest, AdjudicationContext, Kernel, Verdict};
pub use mcp::{McpContext, McpRule, McpViolation};
pub use mcp_audit::{McpAuditLog, McpAuditRecord, McpEnforcementOutcome};
pub use tee::{TeeAttestation, TeePlatform, TeePolicy};
