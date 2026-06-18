#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::{
    sync::{Arc, Mutex},
    thread,
};

use exo_dag_db_domain::lifecycle_action::{
    LifecycleAction, LifecycleActionLedger, LifecycleActionType, LifecycleEvidenceRef,
    LifecycleMemoryRef, LifecycleRollbackRef, LifecycleTerminalState,
    PRD17_LIFECYCLE_ACTION_SCHEMA, ProductionLifecycleApproval,
};

const TENANT: &str = "dag_db-local";
const PROJECT: &str = "dag_db";
const NAMESPACE: &str = "project_memory_v3";

fn memory_ref(memory_id: &str) -> LifecycleMemoryRef {
    LifecycleMemoryRef {
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        memory_id: memory_id.to_owned(),
    }
}

fn action(action_id: &str) -> LifecycleAction {
    let validation_report_id = format!("validation-{action_id}");
    LifecycleAction {
        schema_version: PRD17_LIFECYCLE_ACTION_SCHEMA.to_owned(),
        action_id: action_id.to_owned(),
        action_type: LifecycleActionType::Writeback,
        tenant_id: TENANT.to_owned(),
        project_id: PROJECT.to_owned(),
        memory_namespace: NAMESPACE.to_owned(),
        actor_id: "did:agent:codex-prd17c".to_owned(),
        source_packet_id: "packet-prd17c-concurrency-001".to_owned(),
        source_receipt_id: "receipt-prd17c-concurrency-001".to_owned(),
        parent_memory_ids: vec![memory_ref("memory-parent-a")],
        target_memory_ids: vec![memory_ref("memory-target-a")],
        validation_report_id: validation_report_id.clone(),
        policy_ref: "policy-prd17c-concurrency".to_owned(),
        rollback_ref: LifecycleRollbackRef {
            rollback_id: format!("rollback-{action_id}"),
            action_id: action_id.to_owned(),
            inverse_action_type: LifecycleActionType::Archive,
            before_refs: vec![memory_ref("memory-parent-a")],
            after_refs: vec![memory_ref("memory-target-a")],
            validation_ref: validation_report_id,
            operator_required: true,
        },
        route_invalidation_event_ids: vec!["route-event-prd17c-concurrency-001".to_owned()],
        evidence_refs: vec![LifecycleEvidenceRef {
            evidence_id: "evidence-prd17c-concurrency-001".to_owned(),
            receipt_id: "receipt-evidence-prd17c-concurrency-001".to_owned(),
            digest: "b".repeat(64),
            summary_ref: "summary-evidence-prd17c-concurrency-001".to_owned(),
            preserved: true,
        }],
        terminal_state: LifecycleTerminalState::OperatorDeferred,
        production_lifecycle_approval: ProductionLifecycleApproval::OperatorDeferred,
        created_at: "2026-06-07T00:00:00Z".to_owned(),
    }
}

#[test]
fn concurrent_idempotent_writebacks_commit_once_and_replay_safely() {
    let ledger = Arc::new(Mutex::new(LifecycleActionLedger::default()));
    let handles = (0..8)
        .map(|_| {
            let ledger = Arc::clone(&ledger);
            thread::spawn(move || {
                let action = action("lifecycle-concurrent-writeback-001");
                ledger
                    .lock()
                    .expect("ledger lock")
                    .apply_lifecycle_action(action)
                    .expect("apply lifecycle action")
            })
        })
        .collect::<Vec<_>>();

    let results = handles
        .into_iter()
        .map(|handle| handle.join().expect("join worker"))
        .collect::<Vec<_>>();
    let committed = results.iter().filter(|result| !result.replayed).count();
    let replayed = results.iter().filter(|result| result.replayed).count();

    assert_eq!(committed, 1);
    assert_eq!(replayed, 7);
    assert_eq!(
        ledger.lock().expect("ledger lock").committed_action_count(),
        1
    );
}

#[test]
fn unsafe_concurrent_replay_is_rejected_without_extra_commits() {
    let mut ledger = LifecycleActionLedger::default();
    ledger
        .apply_lifecycle_action(action("lifecycle-concurrent-writeback-001"))
        .expect("seed action");

    let mut unsafe_replay = action("lifecycle-concurrent-writeback-002");
    unsafe_replay.rollback_ref.action_id = unsafe_replay.action_id.clone();
    unsafe_replay.rollback_ref.rollback_id =
        "rollback-lifecycle-concurrent-writeback-002".to_owned();

    assert!(ledger.apply_lifecycle_action(unsafe_replay).is_err());
    assert_eq!(ledger.committed_action_count(), 1);
}
