//! API route definitions — BCTS lifecycle, identity, governance endpoints.
use exo_core::Did;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::error::{GatewayError, Result};
use crate::auth::{Request, authenticate};
use crate::middleware::{Verdict, AuditLog, consent_middleware, governance_middleware, audit_middleware};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Route {
    CreateTransaction, TransitionState, QueryTransaction, GetReceipt,
    ResolveIdentity, RegisterIdentity,
    Deliberate, Vote, Challenge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteResult { pub route: Route, pub status: String, pub correlation_id: Uuid }

/// Process a request through the full middleware chain: auth -> consent -> governance -> execution.
pub fn process_request(request: &Request, route: Route, consent: bool, verdict: &Verdict, log: &mut AuditLog) -> Result<RouteResult> {
    let actor = authenticate(request)?;
    consent_middleware(&actor.did, &request.action, consent)?;
    governance_middleware(&actor.did, &request.action, verdict)?;
    let result = RouteResult { route: route.clone(), status: "ok".into(), correlation_id: Uuid::new_v4() };
    let now = exo_core::Timestamp::now_utc();
    audit_middleware(&actor.did, &request.action, "success", &now, log)?;
    Ok(result)
}

/// Default-deny: a request with no consent is always rejected.
pub fn default_deny_check(actor: &Did, action: &str) -> Result<()> {
    Err(GatewayError::ConsentDenied { reason: format!("default-deny: {actor} cannot {action} without explicit consent") })
}

#[cfg(test)]
mod tests {
    use super::*;
    use exo_core::{Hash256, Signature, Timestamp};
    fn sig() -> Signature { let mut s = [0u8; 64]; s[0] = 1; Signature::from_bytes(s) }
    fn req() -> Request { Request { actor_did: "did:exo:alice".into(), action: "create".into(), body_hash: Hash256::ZERO, signature: sig(), timestamp: Timestamp::ZERO } }

    #[test] fn full_chain_ok() {
        let mut log = AuditLog::new();
        let r = process_request(&req(), Route::CreateTransaction, true, &Verdict::Allow, &mut log);
        assert!(r.is_ok());
        assert_eq!(log.len(), 1);
    }
    #[test] fn auth_fails() {
        let mut log = AuditLog::new();
        let bad = Request { actor_did: "bad".into(), ..req() };
        assert!(process_request(&bad, Route::CreateTransaction, true, &Verdict::Allow, &mut log).is_err());
    }
    #[test] fn consent_fails() {
        let mut log = AuditLog::new();
        assert!(process_request(&req(), Route::CreateTransaction, false, &Verdict::Allow, &mut log).is_err());
    }
    #[test] fn governance_fails() {
        let mut log = AuditLog::new();
        assert!(process_request(&req(), Route::CreateTransaction, true, &Verdict::Deny{reason:"no".into()}, &mut log).is_err());
    }
    #[test] fn default_deny() {
        let did = Did::new("did:exo:alice").unwrap();
        assert!(default_deny_check(&did, "write").is_err());
    }
    #[test] fn route_serde() {
        for r in [Route::CreateTransaction, Route::TransitionState, Route::QueryTransaction, Route::GetReceipt,
                   Route::ResolveIdentity, Route::RegisterIdentity, Route::Deliberate, Route::Vote, Route::Challenge] {
            let j = serde_json::to_string(&r).unwrap(); let rr: Route = serde_json::from_str(&j).unwrap(); assert_eq!(rr, r);
        }
    }
}
