//! Authentication middleware — JWT, OAuth 2.0, SAML support.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Authentication errors.
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token")]
    InvalidToken,
    #[error("Insufficient permissions")]
    InsufficientPermissions,
    #[error("User not found")]
    UserNotFound,
    #[error("Provider error: {0}")]
    ProviderError(String),
}

/// Authentication provider types.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthProvider {
    /// JWT-based authentication.
    Jwt,
    /// OAuth 2.0.
    OAuth2 { provider: String },
    /// SAML for enterprise SSO.
    Saml { idp_entity_id: String },
    /// API key authentication.
    ApiKey,
}

/// An authenticated user session.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthenticatedUser {
    pub user_id: String,
    pub tenant_id: Uuid,
    pub did: String,
    pub roles: Vec<String>,
    pub auth_provider: AuthProvider,
    pub authenticated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl AuthenticatedUser {
    /// Check if the session is still valid.
    pub fn is_valid(&self) -> bool {
        Utc::now() < self.expires_at
    }

    /// Check if the user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.iter().any(|r| r == role)
    }
}

/// A JWT auth token (simplified).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    pub token_type: String,
    pub expires_in_seconds: u64,
    pub refresh_token: Option<String>,
}

/// JWT token service.
pub struct JwtService {
    issuer: String,
    default_ttl_seconds: u64,
}

impl JwtService {
    pub fn new(issuer: String, default_ttl_seconds: u64) -> Self {
        Self {
            issuer,
            default_ttl_seconds,
        }
    }

    /// Issue a new token for an authenticated user.
    pub fn issue_token(&self, user: &AuthenticatedUser) -> AuthToken {
        // In production: sign with RS256/ES256 private key.
        // Stub: create a deterministic token string.
        let token = format!("{}|{}|{}", self.issuer, user.user_id, user.tenant_id);
        AuthToken {
            token,
            token_type: "Bearer".into(),
            expires_in_seconds: self.default_ttl_seconds,
            refresh_token: Some(format!("refresh-{}", user.user_id)),
        }
    }

    /// Validate a token and return claims.
    pub fn validate_token(&self, token: &str) -> Result<TokenClaims, AuthError> {
        // Stub validation: parse the pipe-separated format.
        let parts: Vec<&str> = token.split('|').collect();
        if parts.len() != 3 {
            return Err(AuthError::InvalidToken);
        }
        if parts[0] != self.issuer {
            return Err(AuthError::InvalidToken);
        }
        Ok(TokenClaims {
            user_id: parts[1].to_string(),
            tenant_id: parts[2].to_string(),
            issuer: self.issuer.clone(),
        })
    }
}

/// Decoded JWT claims.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TokenClaims {
    pub user_id: String,
    pub tenant_id: String,
    pub issuer: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_user() -> AuthenticatedUser {
        AuthenticatedUser {
            user_id: "user-1".into(),
            tenant_id: Uuid::new_v4(),
            did: "did:exo:alice".into(),
            roles: vec!["admin".into(), "voter".into()],
            auth_provider: AuthProvider::Jwt,
            authenticated_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(1),
        }
    }

    #[test]
    fn test_user_session_validity() {
        let user = test_user();
        assert!(user.is_valid());
        assert!(user.has_role("admin"));
        assert!(!user.has_role("superadmin"));
    }

    #[test]
    fn test_jwt_issue_and_validate() {
        let service = JwtService::new("decision.forum".into(), 3600);
        let user = test_user();
        let token = service.issue_token(&user);

        assert_eq!(token.token_type, "Bearer");
        let claims = service.validate_token(&token.token).unwrap();
        assert_eq!(claims.user_id, "user-1");
    }

    #[test]
    fn test_invalid_token() {
        let service = JwtService::new("decision.forum".into(), 3600);
        assert!(service.validate_token("bad-token").is_err());
    }
}
