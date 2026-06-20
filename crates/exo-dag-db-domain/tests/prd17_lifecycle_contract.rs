#![allow(clippy::expect_used, clippy::unwrap_used)]

use exo_dag_db_domain::{
    continuation_persistence::{
        ContinuationPersistenceError, ContinuationRecord, ContinuationRetrievalStatus,
        ContinuationStore, PRD17_CONTINUATION_RECORD_SCHEMA,
    },
    lifecycle_action::{
        LifecycleAction, LifecycleActionError, LifecycleActionLedger, LifecycleActionType,
        LifecycleEvidenceRef, LifecycleMemoryRef, LifecycleRollbackRef, LifecycleTerminalState,
        PRD17_LIFECYCLE_ACTION_SCHEMA, PRODUCTION_LIFECYCLE_APPROVAL_EVIDENCE_PREFIX,
        ProductionLifecycleApproval, ProductionLifecycleApprovalEvidence,
    },
    route_invalidation::{
        PRD17_ROUTE_INVALIDATION_EVENT_SCHEMA, RouteFreshnessState, RouteInvalidationError,
        RouteInvalidationEvent, RouteInvalidationLedger, RouteReadinessImpact,
        RouteReadinessRecord,
    },
};
use serde_json::json;

const TENANT: &str = "dag_db-local";
const PROJECT: &str = "dag_db";
const NAMESPACE: &str = "project_memory_v3";

fn digest(byte: &str) -> String {
    byte.repeat(64)
}

fn authority_signature() -> String {
    "0123456789abcdef".repeat(8)
}

fn memory_ref(memory_id: &str) -> LifecycleMemoryRef {
    LifecycleMemoryRef {
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        memory_id: memory_id.to_owned(),
    }
}

fn evidence_ref(evidence_id: &str) -> LifecycleEvidenceRef {
    LifecycleEvidenceRef {
        evidence_id: evidence_id.to_owned(),
        receipt_id: format!("receipt-{evidence_id}"),
        digest: digest("a"),
        summary_ref: format!("summary-{evidence_id}"),
        preserved: true,
    }
}

fn approval_evidence(suffix: &str) -> ProductionLifecycleApprovalEvidence {
    ProductionLifecycleApprovalEvidence {
        evidence_ref: LifecycleEvidenceRef {
            evidence_id: format!("{PRODUCTION_LIFECYCLE_APPROVAL_EVIDENCE_PREFIX}{suffix}"),
            receipt_id: format!("finality-receipt-{suffix}"),
            digest: digest("b"),
            summary_ref: format!("summary-production-approval-{suffix}"),
            preserved: true,
        },
        tenant_id: TENANT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        actor_id: "did:agent:codex-prd17c".to_owned(),
        route_id: "policy-prd17c-local-mutation".to_owned(),
        request_id: "packet-prd17c-001".to_owned(),
        payload_hash: digest("b"),
        authority_did: "did:exo:governance-authority".to_owned(),
        authority_signature: authority_signature(),
        approved_at: "2026-06-07T00:00:01Z".to_owned(),
    }
}

fn continuation_approval_evidence(suffix: &str) -> ProductionLifecycleApprovalEvidence {
    ProductionLifecycleApprovalEvidence {
        request_id: "task-prd17c-next-agent".to_owned(),
        ..approval_evidence(suffix)
    }
}

fn rollback_ref(
    action_id: &str,
    action_type: LifecycleActionType,
    validation_report_id: &str,
) -> LifecycleRollbackRef {
    LifecycleRollbackRef {
        rollback_id: format!("rollback-{action_id}"),
        action_id: action_id.to_owned(),
        inverse_action_type: action_type.inverse(),
        before_refs: vec![memory_ref("memory-parent-a")],
        after_refs: vec![memory_ref("memory-target-a")],
        validation_ref: validation_report_id.to_owned(),
        operator_required: true,
    }
}

fn lifecycle_action(action_id: &str, action_type: LifecycleActionType) -> LifecycleAction {
    let validation_report_id = format!("validation-{action_id}");
    LifecycleAction {
        schema_version: PRD17_LIFECYCLE_ACTION_SCHEMA.to_owned(),
        action_id: action_id.to_owned(),
        action_type,
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        actor_id: "did:agent:codex-prd17c".to_owned(),
        source_packet_id: "packet-prd17c-001".to_owned(),
        source_receipt_id: "receipt-prd17c-001".to_owned(),
        parent_memory_ids: vec![memory_ref("memory-parent-a")],
        target_memory_ids: vec![memory_ref("memory-target-a")],
        validation_report_id: validation_report_id.clone(),
        policy_ref: "policy-prd17c-local-mutation".to_owned(),
        rollback_ref: rollback_ref(action_id, action_type, &validation_report_id),
        route_invalidation_event_ids: vec!["route-event-prd17c-001".to_owned()],
        evidence_refs: vec![evidence_ref("evidence-prd17c-001")],
        terminal_state: LifecycleTerminalState::OperatorDeferred,
        production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
        created_at: "2026-06-07T00:00:00Z".to_owned(),
    }
}

fn route() -> RouteReadinessRecord {
    RouteReadinessRecord {
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        route_id: "route-prd17c-001".to_owned(),
        selected_memory_ids: vec![memory_ref("memory-target-a")],
        freshness_state: RouteFreshnessState::Current,
        last_rebuild_ref: None,
    }
}

fn route_event() -> RouteInvalidationEvent {
    RouteInvalidationEvent {
        schema_version: PRD17_ROUTE_INVALIDATION_EVENT_SCHEMA.to_owned(),
        event_id: "route-event-prd17c-001".to_owned(),
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        route_id: "route-prd17c-001".to_owned(),
        source_action_id: "lifecycle-writeback-001".to_owned(),
        impacted_memory_ids: vec![memory_ref("memory-target-a")],
        reason: "writeback mutation changed selected memory refs".to_owned(),
        invalidated_packet_ids: vec!["context-packet-prd17c-001".to_owned()],
        freshness_state_before: RouteFreshnessState::Current,
        freshness_state_after: RouteFreshnessState::Stale,
        retrieval_readiness_impact: RouteReadinessImpact::RejectUntilRebuilt,
        validation_report_id: "validation-lifecycle-writeback-001".to_owned(),
        rollback_ref: "rollback-lifecycle-writeback-001".to_owned(),
        created_at: "2026-06-07T00:00:00Z".to_owned(),
    }
}

fn continuation() -> ContinuationRecord {
    ContinuationRecord {
        schema_version: PRD17_CONTINUATION_RECORD_SCHEMA.to_owned(),
        continuation_id: "continuation-prd17c-001".to_owned(),
        task_id: "task-prd17c-next-agent".to_owned(),
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        summary_ref: "summary-continuation-prd17c-001".to_owned(),
        memory_refs: vec![memory_ref("memory-target-a")],
        blocker_refs: vec!["blocker-production-lifecycle-approval-deferred".to_owned()],
        validation_refs: vec!["validation-continuation-prd17c-001".to_owned()],
        expiry_epoch_seconds: 2_000,
        later_retrieval_status: ContinuationRetrievalStatus::Pending,
        production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
        created_at: "2026-06-07T00:00:00Z".to_owned(),
    }
}

#[test]
fn lifecycle_action_applies_and_replays_idempotently_without_accepting_production() {
    let action = lifecycle_action("lifecycle-writeback-001", LifecycleActionType::Writeback);
    action.validate().expect("valid lifecycle action");
    assert_eq!(
        action.terminal_state,
        LifecycleTerminalState::OperatorDeferred,
        "missing production approval must remain operator_deferred"
    );

    let mut ledger = LifecycleActionLedger::default();
    let first = ledger
        .apply_lifecycle_action(action.clone())
        .expect("first lifecycle apply");
    assert!(!first.replayed);
    assert_eq!(first.route_invalidation_event_count, 1);

    let replay = ledger
        .apply_lifecycle_action(action)
        .expect("idempotent replay");
    assert!(replay.replayed);
    assert_eq!(ledger.committed_action_count(), 1);
}

#[test]
fn lifecycle_rejects_missing_rollback_ref() {
    let mut raw = serde_json::to_value(lifecycle_action(
        "lifecycle-relink-001",
        LifecycleActionType::Relink,
    ))
    .unwrap();
    raw.as_object_mut()
        .unwrap()
        .remove("rollback_ref")
        .expect("rollback_ref present");

    assert!(matches!(
        LifecycleAction::parse_json(&raw.to_string()),
        Err(LifecycleActionError::Json { .. })
    ));
}

#[test]
fn lifecycle_rejects_missing_validation_ref() {
    let mut action = lifecycle_action("lifecycle-relink-001", LifecycleActionType::Relink);
    action.validation_report_id.clear();

    assert_eq!(
        action.validate(),
        Err(LifecycleActionError::EmptyField {
            field: "validation_report_id".to_owned(),
        })
    );
}

#[test]
fn lifecycle_rejects_cross_tenant_refs() {
    let mut action = lifecycle_action("lifecycle-relink-001", LifecycleActionType::Relink);
    action.target_memory_ids[0].tenant_id = "other-tenant".to_owned();

    assert!(matches!(
        action.validate(),
        Err(LifecycleActionError::ScopeMismatch { .. })
    ));
}

#[test]
fn lifecycle_rejects_raw_body_fields_before_deserialize() {
    let mut raw = serde_json::to_value(lifecycle_action(
        "lifecycle-recycle-001",
        LifecycleActionType::Recycle,
    ))
    .unwrap();
    raw.as_object_mut()
        .unwrap()
        .insert("raw_body".to_owned(), json!("raw output must not persist"));

    assert!(matches!(
        LifecycleAction::parse_json(&raw.to_string()),
        Err(LifecycleActionError::ForbiddenMaterial { .. })
    ));
}

#[test]
fn lifecycle_rejects_archive_or_recycle_that_would_delete_evidence() {
    let mut action = lifecycle_action("lifecycle-recycle-001", LifecycleActionType::Recycle);
    action.evidence_refs[0].preserved = false;

    assert_eq!(
        action.validate(),
        Err(LifecycleActionError::EvidenceWouldBeDeleted {
            action_id: "lifecycle-recycle-001".to_owned(),
        })
    );
}

#[test]
fn lifecycle_rejects_route_invalidation_gaps() {
    let mut action = lifecycle_action("lifecycle-relink-001", LifecycleActionType::Relink);
    action.route_invalidation_event_ids.clear();

    assert_eq!(
        action.validate(),
        Err(LifecycleActionError::RouteInvalidationGap {
            action_id: "lifecycle-relink-001".to_owned(),
        })
    );
}

#[test]
fn lifecycle_rejects_duplicate_unsafe_replay() {
    let action = lifecycle_action("lifecycle-relink-001", LifecycleActionType::Relink);
    let mut replay = action.clone();
    replay.action_id = "lifecycle-relink-002".to_owned();
    replay.rollback_ref.action_id = replay.action_id.clone();
    replay.rollback_ref.rollback_id = "rollback-lifecycle-relink-002".to_owned();

    let mut ledger = LifecycleActionLedger::default();
    ledger
        .apply_lifecycle_action(action)
        .expect("first lifecycle action");

    assert!(matches!(
        ledger.apply_lifecycle_action(replay),
        Err(LifecycleActionError::DuplicateUnsafeReplay { .. })
    ));
}

#[test]
fn lifecycle_approval_evidence_graduates_action_and_replays_idempotently() {
    let action = lifecycle_action("lifecycle-approved-001", LifecycleActionType::Writeback);
    let approval = approval_evidence("lifecycle-approved-001");

    let accepted = action
        .approved_with_evidence(&approval)
        .expect("approval/finality evidence accepts lifecycle action");
    assert_eq!(accepted.terminal_state, LifecycleTerminalState::Accepted);
    assert_eq!(
        accepted.production_lifecycle_approval,
        ProductionLifecycleApproval::Approved
    );
    accepted.validate().expect("accepted action validates");

    let mut ledger = LifecycleActionLedger::default();
    let first = ledger
        .apply_approved_lifecycle_action(action.clone(), &approval)
        .expect("first approved lifecycle apply");
    assert!(!first.replayed);
    assert_eq!(first.terminal_state, LifecycleTerminalState::Accepted);

    let replay = ledger
        .apply_approved_lifecycle_action(action.clone(), &approval)
        .expect("approved lifecycle replay");
    assert!(replay.replayed);
    assert_eq!(ledger.committed_action_count(), 1);

    let changed_approval = approval_evidence("lifecycle-approved-001-changed");
    assert!(matches!(
        ledger.apply_approved_lifecycle_action(action, &changed_approval),
        Err(LifecycleActionError::DuplicateUnsafeReplay { .. })
    ));
}

#[test]
fn lifecycle_approval_evidence_fails_closed_without_finality_binding() {
    let action = lifecycle_action("lifecycle-approved-002", LifecycleActionType::Writeback);
    let mut approval = approval_evidence("lifecycle-approved-002");
    approval.evidence_ref.evidence_id = "self-asserted-production-approval".to_owned();

    assert!(matches!(
        action.approved_with_evidence(&approval),
        Err(LifecycleActionError::ProductionApprovalMissing { .. })
    ));
}

#[test]
fn lifecycle_and_continuation_reject_shaped_placeholder_approval_evidence() {
    let action = lifecycle_action(
        "lifecycle-placeholder-approval-001",
        LifecycleActionType::Writeback,
    );
    let mut placeholder_lifecycle_approval =
        approval_evidence("lifecycle-placeholder-approval-001");
    placeholder_lifecycle_approval.authority_signature = "a".repeat(128);
    assert!(
        action
            .approved_with_evidence(&placeholder_lifecycle_approval)
            .is_err(),
        "lifecycle approval must reject shaped placeholder authority evidence"
    );

    let record = continuation();
    let mut placeholder_continuation_approval =
        continuation_approval_evidence("continuation-placeholder-approval-001");
    placeholder_continuation_approval.authority_signature = "a".repeat(128);
    assert!(
        record
            .approved_with_evidence(&placeholder_continuation_approval, 1_000)
            .is_err(),
        "continuation approval must reject shaped placeholder authority evidence"
    );
}

#[test]
fn lifecycle_approval_rejects_mismatched_scope_hash_and_forged_signature() {
    let action = lifecycle_action("lifecycle-approved-003", LifecycleActionType::Writeback);

    let mut tenant_mismatch = approval_evidence("lifecycle-approved-003");
    tenant_mismatch.tenant_id = "other-tenant".to_owned();
    assert_eq!(
        action.approved_with_evidence(&tenant_mismatch),
        Err(LifecycleActionError::ProductionApprovalMismatch {
            field: "tenant_id".to_owned()
        })
    );

    let mut hash_mismatch = approval_evidence("lifecycle-approved-003");
    hash_mismatch.payload_hash = digest("c");
    assert!(matches!(
        action.approved_with_evidence(&hash_mismatch),
        Err(LifecycleActionError::ProductionApprovalMismatch { .. })
    ));

    let mut forged_signature = approval_evidence("lifecycle-approved-003");
    forged_signature.authority_signature = "not-a-signature".to_owned();
    assert!(matches!(
        action.approved_with_evidence(&forged_signature),
        Err(LifecycleActionError::InvalidAction { .. })
    ));
}

#[test]
fn lifecycle_rejects_raw_accepted_state_with_caller_controlled_approval_evidence() {
    let mut action = lifecycle_action("lifecycle-raw-accepted-001", LifecycleActionType::Writeback);
    let approval = approval_evidence("lifecycle-raw-accepted-001");
    action.evidence_refs.push(approval.evidence_ref);
    action.terminal_state = LifecycleTerminalState::Accepted;
    action.production_lifecycle_approval = ProductionLifecycleApproval::Approved;

    action
        .validate()
        .expect("raw accepted action is structurally durable");
    assert!(matches!(
        LifecycleAction::parse_json(&serde_json::to_string(&action).expect("serialize action")),
        Err(LifecycleActionError::ProductionApprovalMissing { .. })
    ));
    let mut ledger = LifecycleActionLedger::default();
    assert!(matches!(
        ledger.apply_lifecycle_action(action),
        Err(LifecycleActionError::ProductionApprovalMissing { .. })
    ));
}

#[test]
fn lifecycle_rejects_colon_in_scope_fields_keeping_idempotency_keys_unambiguous() {
    // Regression: tenant "a" + project "b:c" and tenant "a:b" + project "c"
    // used to derive the same colon-joined idempotency key, so the second
    // scope was permanently denied as an unsafe replay.
    for (tenant, project) in [
        (TENANT, "dag_db:extra"),
        (format!("{TENANT}:dag_db").as_str(), "extra"),
    ] {
        let mut action = lifecycle_action("lifecycle-colon-001", LifecycleActionType::Writeback);
        action.tenant_id = tenant.to_owned();
        action.project_id = project.to_owned();
        assert!(
            matches!(
                action.validate(),
                Err(LifecycleActionError::ForbiddenMaterial { .. })
            ),
            "colon-bearing scope fields must be rejected: {tenant}/{project}"
        );
    }

    let mut namespaced = lifecycle_action("lifecycle-colon-002", LifecycleActionType::Writeback);
    namespaced.memory_namespace = "project_memory:v3".to_owned();
    assert!(matches!(
        namespaced.validate(),
        Err(LifecycleActionError::ForbiddenMaterial { .. })
    ));
}

#[test]
fn continuation_rejects_colon_in_scope_fields() {
    // task_id participates in the colon-joined idempotency key right next to
    // another free-text component, so it must stay colon-free as well.
    let mutations: [fn(&mut ContinuationRecord); 4] = [
        |record| record.tenant_id = "tenant:a".to_owned(),
        |record| record.project_id = "project:a".to_owned(),
        |record| record.memory_namespace = "namespace:v3".to_owned(),
        |record| record.task_id = "task:prd17c".to_owned(),
    ];
    for mutate in mutations {
        let mut record = continuation();
        mutate(&mut record);
        assert!(matches!(
            record.validate(1_000),
            Err(ContinuationPersistenceError::ForbiddenMaterial { .. })
        ));
    }
}

#[test]
fn route_invalidation_rejects_colon_in_scope_fields() {
    // route_id participates in the colon-joined idempotency key right next to
    // another free-text component, so it must stay colon-free as well.
    let mutations: [fn(&mut RouteInvalidationEvent); 4] = [
        |event| event.tenant_id = "tenant:a".to_owned(),
        |event| event.project_id = "project:a".to_owned(),
        |event| event.memory_namespace = "namespace:v3".to_owned(),
        |event| event.route_id = "route:prd17c".to_owned(),
    ];
    for mutate in mutations {
        let mut event = route_event();
        mutate(&mut event);
        assert!(matches!(
            event.validate(),
            Err(RouteInvalidationError::ForbiddenMaterial { .. })
        ));
    }
}

#[test]
fn lifecycle_rejects_accepted_state_without_production_approval() {
    let mut action = lifecycle_action("lifecycle-archive-001", LifecycleActionType::Archive);
    action.terminal_state = LifecycleTerminalState::Accepted;
    action.production_lifecycle_approval = ProductionLifecycleApproval::OperatorDeferred;

    assert_eq!(
        action.validate(),
        Err(LifecycleActionError::ProductionApprovalMissing {
            action_id: "lifecycle-archive-001".to_owned(),
        })
    );
}

#[test]
fn lifecycle_rejects_accepted_state_from_self_asserted_approval() {
    // A deserialized `Approved` carries no operator-authority binding at this
    // local layer, so it must not be able to mint a production-accepted action.
    let mut action = lifecycle_action("lifecycle-archive-002", LifecycleActionType::Archive);
    action.terminal_state = LifecycleTerminalState::Accepted;
    action.production_lifecycle_approval = ProductionLifecycleApproval::Approved;

    assert_eq!(
        action.validate(),
        Err(LifecycleActionError::ProductionApprovalMissing {
            action_id: "lifecycle-archive-002".to_owned(),
        })
    );
}

#[test]
fn route_invalidation_marks_route_stale_until_rebuilt() {
    let mut ledger = RouteInvalidationLedger::default();
    ledger.insert_route(route()).expect("insert route");
    ledger
        .route("route-prd17c-001")
        .expect("route")
        .ensure_ready_for_retrieval()
        .expect("current route ready");

    let event = route_event();
    let first = ledger
        .apply_route_invalidation(event.clone())
        .expect("route invalidation");
    assert!(!first.replayed);
    assert_eq!(ledger.event_count(), 1);
    assert_eq!(first.freshness_state_after, RouteFreshnessState::Stale);

    assert!(matches!(
        ledger
            .route("route-prd17c-001")
            .expect("route")
            .ensure_ready_for_retrieval(),
        Err(RouteInvalidationError::StaleRoute { .. })
    ));

    let replay = ledger
        .apply_route_invalidation(event)
        .expect("route invalidation replay");
    assert!(replay.replayed);
    assert_eq!(ledger.event_count(), 1);

    ledger
        .route_mut("route-prd17c-001")
        .expect("route mut")
        .rebuild(
            "rebuild-route-prd17c-001".to_owned(),
            "validation-route-rebuild-prd17c-001".to_owned(),
        )
        .expect("rebuild route");
    ledger
        .route("route-prd17c-001")
        .expect("route")
        .ensure_ready_for_retrieval()
        .expect("rebuilt route ready");
}

#[test]
fn route_invalidation_rejects_gap_without_packet_impact() {
    let mut event = route_event();
    event.invalidated_packet_ids.clear();

    assert!(matches!(
        event.validate(),
        Err(RouteInvalidationError::InvalidEvent { .. })
    ));
}

#[test]
fn continuation_persists_and_later_retrieval_consumes_current_record() {
    let mut store = ContinuationStore::default();
    let record = continuation();
    let first = store
        .persist_continuation(record.clone(), 1_000)
        .expect("persist continuation");
    assert!(!first.replayed);

    let replay = store
        .persist_continuation(record, 1_000)
        .expect("continuation replay");
    assert!(replay.replayed);
    assert_eq!(store.record_count(), 1);

    let retrieved = store
        .retrieve_for_task("task-prd17c-next-agent", TENANT, PROJECT, NAMESPACE, 1_000)
        .expect("retrieve continuation");
    assert_eq!(
        retrieved.later_retrieval_status,
        ContinuationRetrievalStatus::Retrieved
    );
}

#[test]
fn approved_continuation_persists_replays_and_retrieves_as_accepted() {
    let mut store = ContinuationStore::default();
    let record = continuation();
    let approval = continuation_approval_evidence("continuation-approved-001");
    let approved = record
        .approved_with_evidence(&approval, 1_000)
        .expect("approve continuation");
    assert!(
        approved.blocker_refs.is_empty(),
        "approval should remove the stale production approval deferred blocker"
    );
    assert!(
        !approved
            .blocker_refs
            .contains(&"blocker-production-lifecycle-approval-deferred".to_owned())
    );
    assert!(
        !approved
            .blocker_refs
            .contains(&"production_lifecycle_approval_deferred".to_owned())
    );

    let first = store
        .persist_approved_continuation(record.clone(), &approval, 1_000)
        .expect("persist approved continuation");
    assert!(!first.replayed);

    let replay = store
        .persist_approved_continuation(record, &approval, 1_000)
        .expect("approved continuation replay");
    assert!(replay.replayed);

    let retrieved = store
        .retrieve_approved_for_task("task-prd17c-next-agent", TENANT, PROJECT, NAMESPACE, 1_000)
        .expect("retrieve approved continuation");
    assert_eq!(
        retrieved.production_lifecycle_approval,
        ProductionLifecycleApproval::Approved
    );
    assert_eq!(
        retrieved.later_retrieval_status,
        ContinuationRetrievalStatus::Retrieved
    );
    assert!(
        !retrieved
            .blocker_refs
            .contains(&"blocker-production-lifecycle-approval-deferred".to_owned())
    );
    assert!(
        !retrieved
            .blocker_refs
            .contains(&"production_lifecycle_approval_deferred".to_owned())
    );
    retrieved
        .validate(1_000)
        .expect("retrieved approved continuation remains valid");
}

#[test]
fn approved_continuation_rejects_missing_approval_and_changed_replay_material() {
    let mut store = ContinuationStore::default();
    let record = continuation();
    store
        .persist_continuation(record.clone(), 1_000)
        .expect("persist deferred continuation");
    assert!(matches!(
        store.retrieve_approved_for_task(
            "task-prd17c-next-agent",
            TENANT,
            PROJECT,
            NAMESPACE,
            1_000
        ),
        Err(ContinuationPersistenceError::ProductionApprovalMissing { .. })
    ));

    let approval = continuation_approval_evidence("continuation-approved-002");
    let mut approved_store = ContinuationStore::default();
    approved_store
        .persist_approved_continuation(record.clone(), &approval, 1_000)
        .expect("persist approved continuation");
    assert!(matches!(
        approved_store.persist_approved_continuation(
            record,
            &continuation_approval_evidence("continuation-approved-002-changed"),
            1_000,
        ),
        Err(ContinuationPersistenceError::DuplicateUnsafeReplay { .. })
    ));
}

#[test]
fn approved_continuation_preserves_unrelated_blockers() {
    let mut record = continuation();
    record.blocker_refs = vec![
        "blocker-production-lifecycle-approval-deferred".to_owned(),
        "blocker-route-readiness".to_owned(),
        "production_lifecycle_approval_deferred".to_owned(),
    ];
    let approved = record
        .approved_with_evidence(
            &continuation_approval_evidence("continuation-blockers-001"),
            1_000,
        )
        .expect("approved continuation");

    assert_eq!(
        approved.blocker_refs,
        vec!["blocker-route-readiness".to_owned()]
    );
    approved
        .validate(1_000)
        .expect("approved continuation keeps unrelated blocker valid");
}

#[test]
fn continuation_rejects_raw_approved_state_with_caller_controlled_validation_refs() {
    let mut record = continuation();
    let approval = continuation_approval_evidence("continuation-raw-approved-001");
    record
        .validation_refs
        .push(approval.evidence_ref.evidence_id.clone());
    record
        .validation_refs
        .push(approval.evidence_ref.receipt_id.clone());
    record.validation_refs.sort();
    record.blocker_refs.clear();
    record.production_lifecycle_approval = ProductionLifecycleApproval::Approved;

    record
        .validate(1_000)
        .expect("raw approved continuation is structurally durable");
    assert!(matches!(
        ContinuationRecord::parse_json(&serde_json::to_string(&record).expect("serialize record")),
        Err(ContinuationPersistenceError::ProductionApprovalMissing { .. })
    ));

    let mut store = ContinuationStore::default();
    assert!(matches!(
        store.persist_continuation(record, 1_000),
        Err(ContinuationPersistenceError::ProductionApprovalMissing { .. })
    ));
}

#[test]
fn continuation_retrieval_is_tenant_scoped() {
    let mut store = ContinuationStore::default();
    let record = continuation();
    store
        .persist_continuation(record, 1_000)
        .expect("persist continuation");

    assert!(matches!(
        store.retrieve_for_task(
            "task-prd17c-next-agent",
            "other-tenant",
            PROJECT,
            NAMESPACE,
            1_000
        ),
        Err(ContinuationPersistenceError::ContinuationNotFound { .. })
    ));
}

#[test]
fn continuation_rejects_cross_tenant_refs() {
    let mut record = continuation();
    record.memory_refs[0].tenant_id = "other-tenant".to_owned();

    assert!(matches!(
        record.validate(1_000),
        Err(ContinuationPersistenceError::ScopeMismatch { .. })
    ));
}

#[test]
fn continuation_rejects_expired_continuation() {
    let mut store = ContinuationStore::default();
    let mut record = continuation();
    record.expiry_epoch_seconds = 999;

    assert_eq!(
        store.persist_continuation(record, 1_000),
        Err(ContinuationPersistenceError::ExpiredContinuation {
            continuation_id: "continuation-prd17c-001".to_owned(),
        })
    );
}

#[test]
fn continuation_rejects_cross_project_refs() {
    let mut record = continuation();
    record.memory_refs[0].project_id = "other-project".to_owned();

    assert!(matches!(
        record.validate(1_000),
        Err(ContinuationPersistenceError::ScopeMismatch { .. })
    ));
}

#[test]
fn continuation_rejects_missing_memory_ref() {
    let mut record = continuation();
    record.memory_refs.clear();

    assert!(matches!(
        record.validate(1_000),
        Err(ContinuationPersistenceError::InvalidRecord { .. })
    ));
}

#[test]
fn continuation_rejects_duplicate_unsafe_replay() {
    let record = continuation();
    let mut replay = record.clone();
    replay.continuation_id = "continuation-prd17c-002".to_owned();
    replay.validation_refs = vec!["validation-continuation-prd17c-002".to_owned()];

    let mut store = ContinuationStore::default();
    store
        .persist_continuation(record, 1_000)
        .expect("persist continuation");

    assert!(matches!(
        store.persist_continuation(replay, 1_000),
        Err(ContinuationPersistenceError::DuplicateUnsafeReplay { .. })
    ));
}
