//! Node API authentication — bearer-token guard for write operations.
//!
//! On startup the node generates a random 256-bit admin token (displayed once
//! in the logs).  Every mutating endpoint (`POST`) requires this token in the
//! `Authorization: Bearer <token>` header.  Read-only endpoints (`GET`) are
//! public — the data on a constitutional network is transparent by design.
//!
//! When the node identity module gains a full DID-document registry, this
//! layer will be upgraded to Ed25519 DID-signature authentication (as already
//! implemented in `exo-gateway/src/auth.rs`).

use std::{
    io::{ErrorKind, Write},
    path::Path,
    sync::Arc,
};

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use zeroize::{Zeroize, Zeroizing};

/// Shared bearer token state for the auth middleware.
#[derive(Clone)]
pub struct BearerAuth {
    /// The expected bearer token (hex-encoded 256-bit random value).
    pub token: Arc<Zeroizing<String>>,
}

/// Generate a cryptographically random admin token (hex-encoded 32 bytes).
#[must_use]
#[allow(clippy::expect_used)] // OS entropy failure is unrecoverable.
pub fn generate_admin_token() -> Zeroizing<String> {
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).expect("OS entropy source unavailable");
    let token = Zeroizing::new(hex::encode(bytes));
    bytes.zeroize();
    token
}

/// Persist an admin token with restrictive file permissions from creation.
///
/// The temporary file is created with `create_new`, and on Unix the `0600` mode
/// is applied atomically during open rather than by chmod after plaintext has
/// already hit the filesystem. The final rename preserves restart behavior by
/// replacing any prior token file.
pub fn write_admin_token_file(path: &Path, token: &str) -> std::io::Result<()> {
    let tmp_path = path.with_extension("tmp");
    match std::fs::remove_file(&tmp_path) {
        Ok(()) => {}
        Err(error) if error.kind() == ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&tmp_path)?;
        file.write_all(token.as_bytes())?;
        file.sync_all()?;
    }

    #[cfg(not(unix))]
    {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp_path)?;
        file.write_all(token.as_bytes())?;
        file.sync_all()?;
    }

    if let Err(error) = std::fs::rename(&tmp_path, path) {
        let _ = std::fs::remove_file(&tmp_path);
        return Err(error);
    }

    Ok(())
}

/// axum middleware: require bearer token on mutating requests.
///
/// `GET` and `HEAD` requests pass through without authentication.
/// All other methods (`POST`, `PUT`, `DELETE`, `PATCH`) require
/// `Authorization: Bearer <token>`.
pub async fn require_bearer_on_writes(
    auth: BearerAuth,
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // Read-only methods pass through.
    let method = request.method().clone();
    if method == axum::http::Method::GET || method == axum::http::Method::HEAD {
        return Ok(next.run(request).await);
    }

    // Extract the Authorization header.
    let header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    match header {
        Some(value) if value.starts_with("Bearer ") => {
            let provided = &value["Bearer ".len()..];
            if constant_time_eq(provided.as_bytes(), auth.token.as_bytes()) {
                Ok(next.run(request).await)
            } else {
                Err(StatusCode::FORBIDDEN)
            }
        }
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}

/// Constant-time byte-slice equality.
///
/// Returns `false` on length mismatch immediately (length is not a
/// secret; the distinguishing side-channel we care about is content).
/// For equal-length slices, performs a branchless XOR-accumulate over
/// every byte so the total work is independent of where the first
/// differing byte is located.
///
/// We inline this instead of pulling in `subtle` or `constant_time_eq`
/// to avoid adding a dependency for one comparison. The implementation
/// is the standard XOR-OR-fold and is sufficient against timing
/// attacks on a bearer-token comparison.
#[inline]
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use axum::{
        Router,
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::{get, post},
    };
    use tower::ServiceExt;

    use super::*;

    fn test_auth() -> BearerAuth {
        BearerAuth {
            token: Arc::new(Zeroizing::new("test-token-abc123".to_string())),
        }
    }

    fn test_app() -> Router {
        let auth = test_auth();
        Router::new()
            .route("/read", get(|| async { "ok" }))
            .route("/write", post(|| async { "ok" }))
            .layer(middleware::from_fn(move |req, next| {
                let a = auth.clone();
                require_bearer_on_writes(a, req, next)
            }))
    }

    #[tokio::test]
    async fn get_requests_pass_without_token() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/read")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_without_token_rejected() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/write")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn post_with_wrong_token_forbidden() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/write")
                    .header("Authorization", "Bearer wrong-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn post_with_correct_token_passes() {
        let app = test_app();
        let resp = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/write")
                    .header("Authorization", "Bearer test-token-abc123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn token_generation_is_unique() {
        let t1 = generate_admin_token();
        let t2 = generate_admin_token();
        assert_ne!(t1, t2);
        assert_eq!(t1.len(), 64); // 32 bytes hex-encoded
    }

    #[test]
    fn admin_token_writer_replaces_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin_token");

        std::fs::write(&path, "old-token").unwrap();
        write_admin_token_file(&path, "new-token").unwrap();

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new-token");
        assert!(!path.with_extension("tmp").exists());
    }

    #[cfg(unix)]
    #[test]
    fn admin_token_writer_creates_owner_only_file() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("admin_token");

        write_admin_token_file(&path, "secret-token").unwrap();

        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "secret-token");
    }

    #[test]
    fn constant_time_eq_matches_equal() {
        assert!(constant_time_eq(b"abcdef", b"abcdef"));
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(&[0u8; 32], &[0u8; 32]));
    }

    #[test]
    fn constant_time_eq_rejects_different() {
        assert!(!constant_time_eq(b"abcdef", b"abcdeg"));
        assert!(!constant_time_eq(b"short", b"different-length"));
        assert!(!constant_time_eq(b"", b"a"));
    }

    #[test]
    fn constant_time_eq_distinguishes_byte_differences() {
        // Difference at the first byte
        assert!(!constant_time_eq(b"xbcdef", b"abcdef"));
        // Difference at the last byte
        assert!(!constant_time_eq(b"abcdex", b"abcdef"));
        // Multiple differences
        assert!(!constant_time_eq(b"xxxxxx", b"abcdef"));
    }

    #[test]
    fn production_source_uses_zeroizing_admin_token_storage() {
        let source = include_str!("auth.rs");
        let production = source
            .split("// ---------------------------------------------------------------------------\n// Tests")
            .next()
            .expect("tests marker present");

        assert!(
            production.contains("Zeroizing<String>"),
            "bearer admin token must be stored in zeroize-on-drop storage"
        );
        assert!(
            !production.contains("Arc<String>"),
            "bearer admin token must not be held in a plain Arc<String>"
        );
        assert!(
            production.contains("bytes.zeroize()"),
            "raw random token bytes must be wiped after hex encoding"
        );
    }

    #[test]
    fn main_persists_admin_token_through_restrictive_auth_writer() {
        let source = include_str!("main.rs");
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("tests marker present");

        assert!(
            production.contains("auth::write_admin_token_file"),
            "startup must persist admin tokens through the restrictive auth writer"
        );
        assert!(
            !production.contains("std::fs::write(&token_path, &admin_token)"),
            "startup must not write the admin token before restrictive permissions are set"
        );
    }
}
