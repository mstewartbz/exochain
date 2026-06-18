#![allow(clippy::expect_used)]

use exo_dag_db_domain::{
    context_packet_persistence::{
        ContextPacketRequest, ContextPacketRouteBinding, DefaultContextQuality,
        PacketFreshnessStatus, PacketPersistenceStatus, PacketValidationStatus,
        build_context_packet_persistence_report, build_context_packet_record,
        canonical_idempotency_key, validate_context_packet_record,
    },
    default_route::{
        DEFAULT_ROUTE_SCHEMA_VERSION, DefaultRetrievalFailureCode, DefaultRouteError,
        DefaultRouteMemoryRef, DefaultRouteRecord, DefaultRouteSource, DefaultRouteStatus,
        DefaultRuntimeReadinessStatus, RouteFreshnessStatus, build_default_context_packet,
        evaluate_default_route_readiness, validate_default_route_record,
    },
};

fn memory_ref(id: &str) -> DefaultRouteMemoryRef {
    DefaultRouteMemoryRef {
        memory_id: id.to_owned(),
        latest_receipt_hash: format!("{id}-receipt"),
        validation_status: "passed".to_owned(),
        citation_ref: format!("citation:{id}"),
    }
}

fn route() -> DefaultRouteRecord {
    DefaultRouteRecord {
        schema_version: DEFAULT_ROUTE_SCHEMA_VERSION.to_owned(),
        route_id: "route-prd17b-default".to_owned(),
        tenant_id: "dag_db-local".to_owned(),
        project_id: "dag_db".to_owned(),
        memory_namespace: "project_memory_v3".to_owned(),
        status: DefaultRouteStatus::Active,
        route_source: DefaultRouteSource::Persisted,
        policy_ref: "policy:prd17b-default-route".to_owned(),
        freshness_ref: "freshness:current".to_owned(),
        policy_allowed: true,
        freshness_status: RouteFreshnessStatus::Current,
        invalidated: false,
        production_default_route_approval_status: "accepted".to_owned(),
        packet_quality_review_status: "accepted".to_owned(),
        selected_memory_refs: vec![memory_ref("memory-a"), memory_ref("memory-b")],
        created_at: "hlc:1".to_owned(),
        updated_at: "hlc:2".to_owned(),
    }
}

fn packet_request() -> ContextPacketRequest {
    ContextPacketRequest {
        packet_id: "packet-prd17b-default".to_owned(),
        query_hash: "query-prd17b-default".to_owned(),
        selected_memory_ids: vec!["memory-a".to_owned(), "memory-b".to_owned()],
        selected_edge_ids: vec!["edge-a-b".to_owned()],
        token_budget: 2_000,
        token_estimate: 1_200,
        citation_coverage_bp: 10_000,
        validation_coverage_bp: 10_000,
        source_proof_refs: vec!["proof:route-readiness".to_owned()],
        context_quality: DefaultContextQuality::UsableContext,
        freshness_status: PacketFreshnessStatus::Current,
        validation_status: PacketValidationStatus::Passed,
        persistence_status: PacketPersistenceStatus::ProofBound,
        fallback_reason: None,
        raw_body_present: false,
        created_at: "hlc:3".to_owned(),
    }
}

fn route_binding() -> ContextPacketRouteBinding {
    ContextPacketRouteBinding {
        route_id: "route-prd17b-default".to_owned(),
        tenant_id: "dag_db-local".to_owned(),
        project_id: "dag_db".to_owned(),
        memory_namespace: "project_memory_v3".to_owned(),
        production_default_route_approval_status: "accepted".to_owned(),
        packet_quality_review_status: "accepted".to_owned(),
        route_freshness_status: PacketFreshnessStatus::Current,
    }
}

#[test]
fn accepted_default_route_builds_deterministic_packet_record() {
    let first = build_default_context_packet(&route(), packet_request())
        .expect("route decision")
        .packet_record
        .expect("accepted route builds packet");
    let second = build_default_context_packet(&route(), packet_request())
        .expect("route decision")
        .packet_record
        .expect("accepted route builds packet");
    assert_eq!(first, second);
    assert_eq!(
        first.idempotency_key,
        canonical_idempotency_key("route-prd17b-default", "query-prd17b-default", 2_000)
    );
    assert_eq!(
        serde_json::to_string(&first).expect("serialize"),
        serde_json::to_string(&second).expect("serialize")
    );
}

#[test]
fn packet_request_cannot_smuggle_memory_ids_outside_the_route() {
    // A requested memory id not bound by the accepted route must be rejected,
    // so callers cannot inject arbitrary memories past the route binding.
    let mut forged = packet_request();
    forged.selected_memory_ids = vec!["memory-a".to_owned(), "memory-not-in-route".to_owned()];
    assert_eq!(
        build_default_context_packet(&route(), forged),
        Err(DefaultRouteError::SelectedMemoryNotInRoute {
            memory_id: "memory-not-in-route".to_owned(),
        })
    );
}

#[test]
fn packet_request_freshness_must_match_accepted_route() {
    // A request claiming a different freshness than the bound route must be
    // rejected, so a fresher request cannot ride an otherwise-bound route.
    let mut mismatched = packet_request();
    mismatched.freshness_status = PacketFreshnessStatus::StaleMemory;
    assert_eq!(
        build_default_context_packet(&route(), mismatched),
        Err(DefaultRouteError::RequestFreshnessOutranksRoute)
    );
}

#[test]
fn missing_operator_approval_is_deferred_not_accepted() {
    let mut deferred = route();
    deferred.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred.packet_quality_review_status = "operator_deferred".to_owned();
    let report = evaluate_default_route_readiness(&deferred).expect("readiness");
    assert_eq!(
        report.readiness_status,
        DefaultRuntimeReadinessStatus::OperatorDeferred
    );
    assert_eq!(
        report.primary_failure_code,
        DefaultRetrievalFailureCode::MissingProductionApproval
    );
    assert!(
        report
            .rejection_reasons
            .contains(&"packet_quality_review_operator_deferred".to_owned())
    );
    let decision =
        build_default_context_packet(&deferred, packet_request()).expect("deferred decision");
    assert!(decision.packet_record.is_none());
    assert_eq!(
        decision.readiness_status,
        DefaultRuntimeReadinessStatus::OperatorDeferred
    );
}

#[test]
fn readiness_rejects_preview_dry_run_stale_forbidden_and_missing_scope() {
    let mut preview = route();
    preview.route_source = DefaultRouteSource::Preview;
    assert_eq!(
        evaluate_default_route_readiness(&preview)
            .expect("preview report")
            .readiness_status,
        DefaultRuntimeReadinessStatus::NonDefault
    );

    let mut dry_run = route();
    dry_run.status = DefaultRouteStatus::DryRunOnly;
    assert_eq!(
        evaluate_default_route_readiness(&dry_run)
            .expect("dry run report")
            .primary_failure_code,
        DefaultRetrievalFailureCode::DryRunOnlyRoute
    );

    let mut stale = route();
    stale.freshness_status = RouteFreshnessStatus::StaleValidation;
    assert_eq!(
        evaluate_default_route_readiness(&stale)
            .expect("stale report")
            .primary_failure_code,
        DefaultRetrievalFailureCode::StaleRoute
    );

    let mut forbidden = route();
    forbidden.status = DefaultRouteStatus::Forbidden;
    assert_eq!(
        evaluate_default_route_readiness(&forbidden)
            .expect("forbidden report")
            .primary_failure_code,
        DefaultRetrievalFailureCode::ForbiddenRoute
    );

    let mut missing_scope = route();
    missing_scope.tenant_id.clear();
    assert!(validate_default_route_record(&missing_scope).is_err());
}

#[test]
fn packet_persistence_rejects_over_budget_empty_low_citation_stale_and_raw() {
    let binding = route_binding();
    let mut over_budget = packet_request();
    over_budget.token_estimate = 2_001;
    assert!(build_context_packet_record(&binding, over_budget).is_err());

    let mut empty = packet_request();
    empty.selected_memory_ids.clear();
    assert!(build_context_packet_record(&binding, empty).is_err());

    let mut low_citation = packet_request();
    low_citation.citation_coverage_bp = 7_999;
    assert!(build_context_packet_record(&binding, low_citation).is_err());

    let mut stale = packet_request();
    stale.freshness_status = PacketFreshnessStatus::StaleValidation;
    assert!(build_context_packet_record(&binding, stale).is_err());

    let mut raw = packet_request();
    raw.raw_body_present = true;
    assert!(build_context_packet_record(&binding, raw).is_err());
}

#[test]
fn packet_report_marks_missing_quality_review_as_operator_deferred() {
    let mut binding = route_binding();
    binding.packet_quality_review_status = "operator_deferred".to_owned();
    let record = build_context_packet_record(&binding, packet_request()).expect("record");
    validate_context_packet_record(&record).expect("record structurally valid");
    let report = build_context_packet_persistence_report(&record);
    assert!(!report.accepted);
    assert!(report.operator_deferred);
    assert!(
        report
            .rejection_reasons
            .contains(&"packet_quality_review_operator_deferred".to_owned())
    );
}
