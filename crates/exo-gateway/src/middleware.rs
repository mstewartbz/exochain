//! Governance middleware chain — consent, governance, and audit middleware.
use exo_core::{Did, Timestamp};
use serde::{Deserialize, Serialize};
use crate::error::{GatewayError, Result};

/// Verdict from governance adjudication.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict { Allow, Deny { reason: String }, Escalate { reason: String } }

/// Audit log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub actor: Did,
    pub action: String,
    pub timestamp: Timestamp,
    pub outcome: String,
}

/// Audit log.
#[derive(Debug, Clone, Default)]
pub struct AuditLog { pub entries: Vec<AuditEntry> }
impl AuditLog {
    #[must_use] pub fn new() -> Self { Self { entries: Vec::new() } }
    pub fn record(&mut self, entry: AuditEntry) { self.entries.push(entry); }
    #[must_use] pub fn len(&self) -> usize { self.entries.len() }
    #[must_use] pub fn is_empty(&self) -> bool { self.entries.is_empty() }
}

/// Consent check — default-deny. Returns Ok if consent is explicitly granted.
pub fn consent_middleware(actor: &Did, _action: &str, consent_granted: bool) -> Result<()> {
    if consent_granted { Ok(()) } else {
        Err(GatewayError::ConsentDenied { reason: format!("no consent for {actor}") })
    }
}

/// Governance check — every action must pass governance adjudication.
pub fn governance_middleware(_actor: &Did, _action: &str, verdict: &Verdict) -> Result<()> {
    match verdict {
        Verdict::Allow => Ok(()),
        Verdict::Deny { reason } => Err(GatewayError::GovernanceDenied { reason: reason.clone() }),
        Verdict::Escalate { reason } => Err(GatewayError::GovernanceDenied { reason: format!("escalated: {reason}") }),
    }
}

/// Record an audit entry for every request.
pub fn audit_middleware(actor: &Did, action: &str, outcome: &str, log: &mut AuditLog) {
    log.record(AuditEntry {
        actor: actor.clone(), action: action.into(),
        timestamp: Timestamp::ZERO, outcome: outcome.into(),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn did(n: &str) -> Did { Did::new(&format!("did:exo:{n}")).unwrap() }

    #[test] fn consent_granted() { assert!(consent_middleware(&did("a"), "read", true).is_ok()); }
    #[test] fn consent_denied() { assert!(consent_middleware(&did("a"), "read", false).is_err()); }
    #[test] fn governance_allow() { assert!(governance_middleware(&did("a"), "r", &Verdict::Allow).is_ok()); }
    #[test] fn governance_deny() { assert!(governance_middleware(&did("a"), "r", &Verdict::Deny{reason:"no".into()}).is_err()); }
    #[test] fn governance_escalate() { assert!(governance_middleware(&did("a"), "r", &Verdict::Escalate{reason:"y".into()}).is_err()); }
    #[test] fn audit_records() { let mut log = AuditLog::new(); audit_middleware(&did("a"), "read", "ok", &mut log); assert_eq!(log.len(), 1); }
    #[test] fn audit_empty() { assert!(AuditLog::new().is_empty()); }
    #[test] fn audit_default() { assert!(AuditLog::default().is_empty()); }
    #[test] fn verdict_serde() {
        for v in [Verdict::Allow, Verdict::Deny{reason:"x".into()}, Verdict::Escalate{reason:"y".into()}] {
            let j = serde_json::to_string(&v).unwrap(); let r: Verdict = serde_json::from_str(&j).unwrap(); assert_eq!(r, v);
        }
    }
}
