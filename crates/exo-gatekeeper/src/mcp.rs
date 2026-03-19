//! Model Context Protocol (MCP) enforcement.
//!
//! Ensures AI systems operating within the EXOCHAIN fabric respect
//! constitutional boundaries on autonomy, identity, and consent.

use exo_core::Did;
use serde::{Deserialize, Serialize};

use crate::types::PermissionSet;

// ---------------------------------------------------------------------------
// MCP rules
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum McpRule {
    Mcp001BctsScope,
    Mcp002NoSelfEscalation,
    Mcp003ProvenanceRequired,
    Mcp004NoIdentityForge,
    Mcp005Distinguishable,
    Mcp006ConsentBoundaries,
}

impl McpRule {
    #[must_use]
    pub fn all() -> Vec<McpRule> {
        vec![
            McpRule::Mcp001BctsScope, McpRule::Mcp002NoSelfEscalation,
            McpRule::Mcp003ProvenanceRequired, McpRule::Mcp004NoIdentityForge,
            McpRule::Mcp005Distinguishable, McpRule::Mcp006ConsentBoundaries,
        ]
    }

    #[must_use]
    pub fn description(&self) -> &'static str {
        match self {
            McpRule::Mcp001BctsScope => "AI must operate within BCTS scope",
            McpRule::Mcp002NoSelfEscalation => "AI cannot self-escalate capabilities",
            McpRule::Mcp003ProvenanceRequired => "AI actions require provenance metadata",
            McpRule::Mcp004NoIdentityForge => "AI cannot forge identity or signatures",
            McpRule::Mcp005Distinguishable => "AI outputs must be distinguishable from human",
            McpRule::Mcp006ConsentBoundaries => "AI must respect consent boundaries",
        }
    }
}

// ---------------------------------------------------------------------------
// MCP context
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct McpContext {
    pub actor_did: Did,
    pub is_ai: bool,
    pub bcts_scope: Option<String>,
    pub capabilities: PermissionSet,
    pub action: String,
    pub has_provenance: bool,
    pub forging_identity: bool,
    pub output_marked_ai: bool,
    pub consent_active: bool,
    pub self_escalation: bool,
}

// ---------------------------------------------------------------------------
// MCP violation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpViolation {
    pub rule: McpRule,
    pub description: String,
    pub evidence: Vec<String>,
    pub severity: u8,
}

// ---------------------------------------------------------------------------
// Enforcement
// ---------------------------------------------------------------------------

pub fn enforce(rules: &[McpRule], context: &McpContext) -> Result<(), McpViolation> {
    if !context.is_ai { return Ok(()); }
    for rule in rules { check_rule(*rule, context)?; }
    Ok(())
}

fn check_rule(rule: McpRule, ctx: &McpContext) -> Result<(), McpViolation> {
    match rule {
        McpRule::Mcp001BctsScope => {
            if ctx.bcts_scope.is_none() {
                return Err(McpViolation { rule, description: "AI operating outside BCTS scope".into(), evidence: vec![format!("actor: {}", ctx.actor_did)], severity: 5 });
            }
            Ok(())
        }
        McpRule::Mcp002NoSelfEscalation => {
            if ctx.self_escalation {
                return Err(McpViolation { rule, description: "AI attempted self-escalation".into(), evidence: vec![format!("actor: {}", ctx.actor_did), format!("action: {}", ctx.action)], severity: 5 });
            }
            Ok(())
        }
        McpRule::Mcp003ProvenanceRequired => {
            if !ctx.has_provenance {
                return Err(McpViolation { rule, description: "AI action lacks provenance".into(), evidence: vec![format!("action: {}", ctx.action)], severity: 4 });
            }
            Ok(())
        }
        McpRule::Mcp004NoIdentityForge => {
            if ctx.forging_identity {
                return Err(McpViolation { rule, description: "AI attempted identity forge".into(), evidence: vec![format!("actor: {}", ctx.actor_did)], severity: 5 });
            }
            Ok(())
        }
        McpRule::Mcp005Distinguishable => {
            if !ctx.output_marked_ai {
                return Err(McpViolation { rule, description: "AI output not marked".into(), evidence: vec![format!("action: {}", ctx.action)], severity: 3 });
            }
            Ok(())
        }
        McpRule::Mcp006ConsentBoundaries => {
            if !ctx.consent_active {
                return Err(McpViolation { rule, description: "AI operating without consent".into(), evidence: vec![format!("actor: {}", ctx.actor_did)], severity: 5 });
            }
            Ok(())
        }
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Permission;

    fn did(s: &str) -> Did { Did::new(s).expect("valid DID") }

    fn valid_ai() -> McpContext {
        McpContext {
            actor_did: did("did:exo:ai-agent-1"),
            is_ai: true,
            bcts_scope: Some("data:medical".into()),
            capabilities: PermissionSet::new(vec![Permission::new("read")]),
            action: "summarize".into(),
            has_provenance: true,
            forging_identity: false,
            output_marked_ai: true,
            consent_active: true,
            self_escalation: false,
        }
    }

    fn human() -> McpContext {
        McpContext {
            actor_did: did("did:exo:human-1"), is_ai: false, bcts_scope: None,
            capabilities: PermissionSet::default(), action: "anything".into(),
            has_provenance: false, forging_identity: false, output_marked_ai: false,
            consent_active: false, self_escalation: false,
        }
    }

    #[test] fn all_pass_valid_ai() { assert!(enforce(&McpRule::all(), &valid_ai()).is_ok()); }
    #[test] fn human_exempt() { assert!(enforce(&McpRule::all(), &human()).is_ok()); }

    #[test] fn mcp001_fail() { let mut c = valid_ai(); c.bcts_scope = None; let e = enforce(&[McpRule::Mcp001BctsScope], &c).unwrap_err(); assert_eq!(e.rule, McpRule::Mcp001BctsScope); assert_eq!(e.severity, 5); }
    #[test] fn mcp001_pass() { assert!(enforce(&[McpRule::Mcp001BctsScope], &valid_ai()).is_ok()); }

    #[test] fn mcp002_fail() { let mut c = valid_ai(); c.self_escalation = true; assert_eq!(enforce(&[McpRule::Mcp002NoSelfEscalation], &c).unwrap_err().rule, McpRule::Mcp002NoSelfEscalation); }
    #[test] fn mcp002_pass() { assert!(enforce(&[McpRule::Mcp002NoSelfEscalation], &valid_ai()).is_ok()); }

    #[test] fn mcp003_fail() { let mut c = valid_ai(); c.has_provenance = false; let e = enforce(&[McpRule::Mcp003ProvenanceRequired], &c).unwrap_err(); assert_eq!(e.rule, McpRule::Mcp003ProvenanceRequired); assert_eq!(e.severity, 4); }
    #[test] fn mcp003_pass() { assert!(enforce(&[McpRule::Mcp003ProvenanceRequired], &valid_ai()).is_ok()); }

    #[test] fn mcp004_fail() { let mut c = valid_ai(); c.forging_identity = true; assert_eq!(enforce(&[McpRule::Mcp004NoIdentityForge], &c).unwrap_err().rule, McpRule::Mcp004NoIdentityForge); }
    #[test] fn mcp004_pass() { assert!(enforce(&[McpRule::Mcp004NoIdentityForge], &valid_ai()).is_ok()); }

    #[test] fn mcp005_fail() { let mut c = valid_ai(); c.output_marked_ai = false; let e = enforce(&[McpRule::Mcp005Distinguishable], &c).unwrap_err(); assert_eq!(e.rule, McpRule::Mcp005Distinguishable); assert_eq!(e.severity, 3); }
    #[test] fn mcp005_pass() { assert!(enforce(&[McpRule::Mcp005Distinguishable], &valid_ai()).is_ok()); }

    #[test] fn mcp006_fail() { let mut c = valid_ai(); c.consent_active = false; assert_eq!(enforce(&[McpRule::Mcp006ConsentBoundaries], &c).unwrap_err().rule, McpRule::Mcp006ConsentBoundaries); }
    #[test] fn mcp006_pass() { assert!(enforce(&[McpRule::Mcp006ConsentBoundaries], &valid_ai()).is_ok()); }

    #[test] fn first_violation_returned() { let mut c = valid_ai(); c.bcts_scope = None; c.has_provenance = false; assert_eq!(enforce(&[McpRule::Mcp001BctsScope, McpRule::Mcp003ProvenanceRequired], &c).unwrap_err().rule, McpRule::Mcp001BctsScope); }
    #[test] fn empty_rules_pass() { assert!(enforce(&[], &valid_ai()).is_ok()); }
    #[test] fn all_six_rules() { assert_eq!(McpRule::all().len(), 6); }
    #[test] fn descriptions_non_empty() { for r in McpRule::all() { assert!(!r.description().is_empty()); } }
}
