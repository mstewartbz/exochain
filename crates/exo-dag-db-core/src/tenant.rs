//! Canonical tenant/namespace identity for DAG DB storage isolation (GAP-012
//! P1-E).
//!
//! Tenant isolation in DAG DB is enforced at the storage layer by a free-text
//! `tenant_id`/`namespace` pair carried on every row. Two hazards follow from
//! that: (1) unvalidated free text lets malformed or ambiguous ids reach the
//! database, and (2) cosmetic spelling drift (`dag-db-local` vs `dag_db-local`)
//! silently partitions what is meant to be one tenant into two. This module is
//! the single source of truth that closes both: a canonical local-dev constant
//! every module routes through, and a fail-closed validator applied at write
//! entrypoints so no new divergence can be introduced.
//!
//! This module performs no I/O and is deterministic.

use thiserror::Error;

/// Canonical local-dev tenant id.
///
/// Chosen as the underscore form because it is what the shipping write paths
/// (`exo_gateway::dagdb` local-dev mount and `continuation_packet`) and the
/// `tools/start_dagdb_local.sh` launch default already persist. The hyphen
/// variant (`dag-db-local`) only ever appears in test fixtures and one smoke
/// binary; routing every module through this constant prevents future writes
/// from diverging. Reconciliation of any already-written hyphen rows is tracked
/// as a follow-up (see `docs/dagdb/INTEGRATION.md`); it is intentionally not a
/// destructive rewrite, because corpus memory is append-only.
pub const LOCAL_DEV_TENANT_ID: &str = "dag_db-local";

/// Canonical local-dev namespace paired with [`LOCAL_DEV_TENANT_ID`].
pub const LOCAL_DEV_NAMESPACE: &str = "dag_db";

/// Maximum accepted length, in bytes, of a tenant id or namespace. Bounds the
/// storage column and rejects pathological inputs; comfortably above every
/// real value in use (UUIDs, slugs, DIDs).
pub const MAX_TENANT_FIELD_LEN: usize = 128;

/// Reasons a tenant id or namespace was rejected on write.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum TenantIdentityError {
    /// The value was empty or whitespace-only.
    #[error("tenant_identity_empty: {field}")]
    Empty {
        /// Field name (`tenant_id` or `namespace`).
        field: &'static str,
    },
    /// The value exceeded [`MAX_TENANT_FIELD_LEN`].
    #[error("tenant_identity_too_long: {field}")]
    TooLong {
        /// Field name.
        field: &'static str,
    },
    /// The value contained a character outside the accepted charset.
    #[error("tenant_identity_invalid_char: {field}")]
    InvalidChar {
        /// Field name.
        field: &'static str,
    },
}

/// Validate and normalize a tenant id for write.
///
/// Returns the normalized value (surrounding whitespace trimmed). Fails closed
/// when empty, over-length, or outside the accepted charset
/// `[A-Za-z0-9_:.-]`. The accepted charset is the union of every real tenant id
/// in use (slugs, UUIDs, DIDs) and deliberately excludes whitespace and SQL/
/// glob metacharacters so a tenant id can never be ambiguous or smuggle a
/// predicate.
pub fn normalize_tenant_id(value: &str) -> Result<String, TenantIdentityError> {
    normalize_field("tenant_id", value)
}

/// Validate and normalize a namespace for write. Same rules as
/// [`normalize_tenant_id`].
pub fn normalize_namespace(value: &str) -> Result<String, TenantIdentityError> {
    normalize_field("namespace", value)
}

fn normalize_field(field: &'static str, value: &str) -> Result<String, TenantIdentityError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(TenantIdentityError::Empty { field });
    }
    if trimmed.len() > MAX_TENANT_FIELD_LEN {
        return Err(TenantIdentityError::TooLong { field });
    }
    if !trimmed.bytes().all(is_accepted_byte) {
        return Err(TenantIdentityError::InvalidChar { field });
    }
    Ok(trimmed.to_owned())
}

const fn is_accepted_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b':' | b'.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_local_dev_identity() {
        assert_eq!(
            normalize_tenant_id(LOCAL_DEV_TENANT_ID).expect("canonical tenant accepted"),
            LOCAL_DEV_TENANT_ID
        );
        assert_eq!(
            normalize_namespace(LOCAL_DEV_NAMESPACE).expect("canonical namespace accepted"),
            LOCAL_DEV_NAMESPACE
        );
    }

    #[test]
    fn accepts_real_world_identity_shapes() {
        for value in [
            "tenant-a",
            "tenant_benchmark",
            "tenant:a",
            "00000000-0000-0000-0000-000000000101",
            "did:exo:dagdb-mcp-local",
            "the-team-local",
        ] {
            assert_eq!(
                normalize_tenant_id(value).expect("real-world tenant accepted"),
                value
            );
        }
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            normalize_tenant_id("  dag_db-local  ").expect("trimmed tenant accepted"),
            "dag_db-local"
        );
    }

    #[test]
    fn rejects_empty_and_whitespace_only() {
        assert!(matches!(
            normalize_tenant_id(""),
            Err(TenantIdentityError::Empty { field: "tenant_id" })
        ));
        assert!(matches!(
            normalize_namespace("   "),
            Err(TenantIdentityError::Empty { field: "namespace" })
        ));
    }

    #[test]
    fn rejects_charset_violations() {
        for bad in [
            "dag db",         // space
            "tenant/a",       // slash
            "tenant%a",       // glob/like metacharacter
            "tenant'; DROP",  // quote + space
            "ten\nant",       // embedded newline (not trimmable)
            "tenant\u{00e9}", // non-ascii
        ] {
            assert!(
                matches!(
                    normalize_tenant_id(bad),
                    Err(TenantIdentityError::InvalidChar { .. })
                ),
                "expected rejection for {bad:?}"
            );
        }
    }

    #[test]
    fn rejects_over_length() {
        let long = "a".repeat(MAX_TENANT_FIELD_LEN + 1);
        assert!(matches!(
            normalize_tenant_id(&long),
            Err(TenantIdentityError::TooLong { field: "tenant_id" })
        ));
    }
}
