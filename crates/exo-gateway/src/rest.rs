//! REST endpoints for enterprise integration (SSO, ERP).

use serde::{Deserialize, Serialize};

/// Health check response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// REST API route definitions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RestRoute {
    /// GET /health
    Health,
    /// GET /api/v1/decisions/:id
    GetDecision,
    /// POST /api/v1/decisions
    CreateDecision,
    /// POST /api/v1/auth/token
    AuthToken,
    /// POST /api/v1/auth/saml/callback
    SamlCallback,
    /// GET /api/v1/tenants/:id/constitution
    GetConstitution,
    /// POST /api/v1/ediscovery/export
    EDiscoveryExport,
    /// GET /api/v1/audit/:decision_id
    AuditTrail,
}

impl RestRoute {
    /// Get the HTTP method for this route.
    pub fn method(&self) -> &str {
        match self {
            RestRoute::Health
            | RestRoute::GetDecision
            | RestRoute::GetConstitution
            | RestRoute::AuditTrail => "GET",
            RestRoute::CreateDecision
            | RestRoute::AuthToken
            | RestRoute::SamlCallback
            | RestRoute::EDiscoveryExport => "POST",
        }
    }

    /// Get the path pattern for this route.
    pub fn path(&self) -> &str {
        match self {
            RestRoute::Health => "/health",
            RestRoute::GetDecision => "/api/v1/decisions/:id",
            RestRoute::CreateDecision => "/api/v1/decisions",
            RestRoute::AuthToken => "/api/v1/auth/token",
            RestRoute::SamlCallback => "/api/v1/auth/saml/callback",
            RestRoute::GetConstitution => "/api/v1/tenants/:id/constitution",
            RestRoute::EDiscoveryExport => "/api/v1/ediscovery/export",
            RestRoute::AuditTrail => "/api/v1/audit/:decision_id",
        }
    }

    /// All defined routes.
    pub fn all() -> Vec<RestRoute> {
        vec![
            RestRoute::Health,
            RestRoute::GetDecision,
            RestRoute::CreateDecision,
            RestRoute::AuthToken,
            RestRoute::SamlCallback,
            RestRoute::GetConstitution,
            RestRoute::EDiscoveryExport,
            RestRoute::AuditTrail,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_methods() {
        assert_eq!(RestRoute::Health.method(), "GET");
        assert_eq!(RestRoute::CreateDecision.method(), "POST");
    }

    #[test]
    fn test_route_paths() {
        assert_eq!(RestRoute::Health.path(), "/health");
        assert!(RestRoute::GetDecision.path().contains(":id"));
    }

    #[test]
    fn test_all_routes() {
        let routes = RestRoute::all();
        assert_eq!(routes.len(), 8);
    }
}
