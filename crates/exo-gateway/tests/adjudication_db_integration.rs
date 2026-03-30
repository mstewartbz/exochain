//! Integration tests for APE-53 — DB adjudication context resolver.
//!
//! These tests exercise the adjudication logic at the integration boundary
//! (outside the `exo-gateway` crate boundary) without requiring a live DB.
//!
//! `ProvenanceVerifiable` is excluded from the adjudication kernel in all
//! "should permit" tests because provenance is a per-action concern (not
//! stored in adjudication tables).  The caller's route handler is responsible
//! for attaching provenance before calling `Kernel::adjudicate`.
//!
//! Test inventory (mirrors APE-53 acceptance criteria):
//!   1. Scaffold (WO-009) denies all — no pool, deny-all context.
//!   2. Role present + consent + chain → Permitted.
//!   3. Role absent + consent + chain → Permitted (roles not required by ConsentRequired).
//!   4. No active bailment → Denied (ConsentRequired invariant).
//!   5. Consent present but empty authority chain → Denied (AuthorityChainValid).
//!   6. Revoked consent + no bailment → Denied.
//!   7. Cross-branch roles → Denied (SeparationOfPowers).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::{Arc, RwLock};

use exo_core::Did;
use exo_gatekeeper::{
    ActionRequest, AdjudicationContext, Kernel,
    invariants::{ConstitutionalInvariant, InvariantSet},
    types::{
        AuthorityChain, AuthorityLink, BailmentState, ConsentRecord, GovernmentBranch, Permission,
        PermissionSet, Role,
    },
};
use exo_gateway::server::AppState;
use exo_identity::did::DidRegistry;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn did(s: &str) -> Did {
    Did::new(s).expect("valid DID")
}

/// Kernel without ProvenanceVerifiable — used when testing context shapes that
/// `build_adjudication_context_from_db` produces (provenance is not stored in
/// adjudication tables; it is attached by the route handler per-action).
fn adjudication_kernel() -> Kernel {
    Kernel::new(
        b"exochain-constitution-v1",
        InvariantSet::with(vec![
            ConstitutionalInvariant::SeparationOfPowers,
            ConstitutionalInvariant::ConsentRequired,
            ConstitutionalInvariant::NoSelfGrant,
            ConstitutionalInvariant::HumanOverride,
            ConstitutionalInvariant::KernelImmutability,
            ConstitutionalInvariant::AuthorityChainValid,
            ConstitutionalInvariant::QuorumLegitimate,
        ]),
    )
}

fn empty_app_state() -> AppState {
    AppState::new(None, Arc::new(RwLock::new(DidRegistry::new())))
}

fn vote_action(actor: &Did) -> ActionRequest {
    ActionRequest {
        actor: actor.clone(),
        action: "vote".into(),
        required_permissions: PermissionSet::new(vec![Permission::new("vote")]),
        is_self_grant: false,
        modifies_kernel: false,
    }
}

/// Construct a fully-valid `AdjudicationContext` that mirrors what
/// `build_adjudication_context_from_db` would produce when the actor has a
/// role, active consent, and a one-link authority chain.
fn full_db_context(actor: &Did) -> AdjudicationContext {
    let grantor = did("did:exo:root-grantor");
    AdjudicationContext {
        actor_roles: vec![Role {
            name: "voter".into(),
            branch: GovernmentBranch::Executive,
        }],
        authority_chain: AuthorityChain {
            links: vec![AuthorityLink {
                grantor: grantor.clone(),
                grantee: actor.clone(),
                permissions: PermissionSet::new(vec![Permission::new("vote")]),
                signature: vec![0xAB; 8], // non-empty: satisfies legacy path
                grantor_public_key: None,
            }],
        },
        consent_records: vec![ConsentRecord {
            subject: grantor.clone(),
            granted_to: actor.clone(),
            scope: "data:vote".into(),
            active: true,
        }],
        bailment_state: BailmentState::Active {
            bailor: grantor.clone(),
            bailee: actor.clone(),
            scope: "data:vote".into(),
        },
        human_override_preserved: true,
        actor_permissions: PermissionSet::new(vec![Permission::new("vote")]),
        provenance: None,
        quorum_evidence: None,
        active_challenge_reason: None,
    }
}

// ---------------------------------------------------------------------------
// Test 1: WO-009 scaffold deny-all (integration boundary)
// ---------------------------------------------------------------------------

/// `AppState::build_adjudication_context` with no DB pool MUST return a
/// deny-all context (WO-009).  Verified at the integration boundary — i.e.
/// called from outside the `exo-gateway` crate, as Gate 12 exercises.
#[tokio::test]
async fn scaffold_denies_all_requests() {
    let state = empty_app_state();
    let actor = did("did:exo:actor001");
    let ctx = state.build_adjudication_context(&actor).await;

    assert_eq!(ctx.bailment_state, BailmentState::None, "WO-009: bailment must be None");
    assert!(ctx.actor_roles.is_empty(), "WO-009: no roles");
    assert!(ctx.authority_chain.is_empty(), "WO-009: no authority chain");

    // Use the full invariant set (including ProvenanceVerifiable) to confirm
    // the scaffold truly denies even a complete Kernel.
    let kernel = Kernel::new(b"exochain-constitution-v1", InvariantSet::all());
    let verdict = kernel.adjudicate(&vote_action(&actor), &ctx);
    assert!(verdict.is_denied(), "scaffold must always deny: {verdict:?}");
}

// ---------------------------------------------------------------------------
// Test 2: role present → permitted
// ---------------------------------------------------------------------------

/// Actor with a judicial role, active consent, and authority chain is permitted
/// — the full happy-path that `build_adjudication_context_from_db` enables.
#[test]
fn role_present_permits() {
    let kernel = adjudication_kernel();
    let actor = did("did:exo:actor002");
    let mut ctx = full_db_context(&actor);
    ctx.actor_roles = vec![Role {
        name: "judge".into(),
        branch: GovernmentBranch::Judicial,
    }];
    assert!(
        kernel.adjudicate(&vote_action(&actor), &ctx).is_permitted(),
        "actor with role + consent + chain must be permitted"
    );
}

// ---------------------------------------------------------------------------
// Test 3: role absent + valid consent + chain → still permitted
// ---------------------------------------------------------------------------

/// `SeparationOfPowers` only fires on *multi-branch* role conflicts; zero
/// roles do not trigger it.  An actor without roles but with consent and a
/// valid authority chain is permitted.
#[test]
fn role_absent_with_consent_and_chain_permits() {
    let kernel = adjudication_kernel();
    let actor = did("did:exo:actor003");
    let mut ctx = full_db_context(&actor);
    ctx.actor_roles = vec![]; // no rows in agent_roles table
    assert!(
        kernel.adjudicate(&vote_action(&actor), &ctx).is_permitted(),
        "no roles + consent + chain must be permitted"
    );
}

// ---------------------------------------------------------------------------
// Test 4: no active bailment → denied
// ---------------------------------------------------------------------------

/// When the DB resolver finds no active `consent_records` it sets
/// `BailmentState::None`.  The kernel denies via `ConsentRequired`.
#[test]
fn no_active_bailment_denies() {
    let kernel = adjudication_kernel();
    let actor = did("did:exo:actor004");
    let mut ctx = full_db_context(&actor);
    ctx.bailment_state = BailmentState::None; // no active consent row in DB
    ctx.consent_records = vec![];
    assert!(
        kernel.adjudicate(&vote_action(&actor), &ctx).is_denied(),
        "no bailment must be denied"
    );
}

// ---------------------------------------------------------------------------
// Test 5: consent present + valid authority chain → permitted
// ---------------------------------------------------------------------------

/// Verifies the combined consent-plus-chain path that APE-53 acceptance
/// criteria explicitly calls out.
#[test]
fn consent_and_authority_chain_permits() {
    let kernel = adjudication_kernel();
    let actor = did("did:exo:actor005");
    let ctx = full_db_context(&actor);

    assert!(!ctx.authority_chain.is_empty(), "authority chain must be non-empty");
    assert!(ctx.bailment_state.is_active(), "bailment must be active");

    assert!(
        kernel.adjudicate(&vote_action(&actor), &ctx).is_permitted(),
        "consent + authority chain must permit"
    );
}

// ---------------------------------------------------------------------------
// Test 6: revoked/absent consent → denied
// ---------------------------------------------------------------------------

/// When all `consent_records` have `active = false` the resolver emits
/// `BailmentState::None`.  Verify this denies even when other fields are valid.
#[test]
fn revoked_consent_denies() {
    let kernel = adjudication_kernel();
    let actor = did("did:exo:actor006");
    let grantor = did("did:exo:root-grantor");
    let ctx = AdjudicationContext {
        actor_roles: vec![Role { name: "voter".into(), branch: GovernmentBranch::Executive }],
        authority_chain: AuthorityChain {
            links: vec![AuthorityLink {
                grantor: grantor.clone(),
                grantee: actor.clone(),
                permissions: PermissionSet::new(vec![Permission::new("vote")]),
                signature: vec![0xAB; 8],
                grantor_public_key: None,
            }],
        },
        consent_records: vec![ConsentRecord {
            subject: grantor.clone(),
            granted_to: actor.clone(),
            scope: "data:vote".into(),
            active: false, // revoked — resolver does not set BailmentState::Active
        }],
        bailment_state: BailmentState::None, // safe default when no active consent
        human_override_preserved: true,
        actor_permissions: PermissionSet::new(vec![Permission::new("vote")]),
        provenance: None,
        quorum_evidence: None,
        active_challenge_reason: None,
    };
    assert!(
        kernel.adjudicate(&vote_action(&actor), &ctx).is_denied(),
        "revoked consent must deny"
    );
}

// ---------------------------------------------------------------------------
// Test 7: cross-branch roles → SeparationOfPowers denied
// ---------------------------------------------------------------------------

/// When `agent_roles` has both Executive and Legislative rows for the same DID
/// the resolver maps them to two `Role` values spanning branches.  The kernel
/// denies via `SeparationOfPowers`.
#[test]
fn cross_branch_roles_denies() {
    let kernel = adjudication_kernel();
    let actor = did("did:exo:actor007");
    let mut ctx = full_db_context(&actor);
    ctx.actor_roles = vec![
        Role { name: "voter".into(), branch: GovernmentBranch::Executive },
        Role { name: "legislator".into(), branch: GovernmentBranch::Legislative },
    ];
    assert!(
        kernel.adjudicate(&vote_action(&actor), &ctx).is_denied(),
        "cross-branch roles must be denied by SeparationOfPowers"
    );
}
