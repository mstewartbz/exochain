#![allow(clippy::expect_used)]

use exo_dag_db_domain::{
    context_packet_persistence::{
        ContextPacketAcceptanceEvidence, ContextPacketRequest, ContextPacketRouteBinding,
        DefaultContextQuality, PacketFreshnessStatus, PacketPersistenceStatus,
        PacketValidationStatus, accept_context_packet_record,
        build_context_packet_persistence_report, build_context_packet_record,
        canonical_idempotency_key, validate_context_packet_record,
    },
    default_route::{
        DEFAULT_ROUTE_SCHEMA_VERSION, DefaultRetrievalFailureCode, DefaultRouteAcceptanceEvidence,
        DefaultRouteError, DefaultRouteMemoryRef, DefaultRouteRecord, DefaultRouteSource,
        DefaultRouteStatus, DefaultRuntimeReadinessStatus, RouteFreshnessStatus,
        accept_default_route_record, build_default_context_packet,
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

fn digest(byte: &str) -> String {
    byte.repeat(64)
}

fn authority_signature() -> String {
    "0123456789abcdef".repeat(8)
}

fn route_acceptance_evidence() -> DefaultRouteAcceptanceEvidence {
    DefaultRouteAcceptanceEvidence {
        production_default_route_approval_ref: "external-production-approval:default-route-prd17b"
            .to_owned(),
        packet_quality_review_ref: "external-packet-quality-review:prd17b".to_owned(),
        finality_ref: "external-finality:default-route-prd17b".to_owned(),
        tenant_id: "dag_db-local".to_owned(),
        memory_namespace: "project_memory_v3".to_owned(),
        actor_id: "did:exo:codex-prd17b".to_owned(),
        route_id: "route-prd17b-default".to_owned(),
        request_id: "request-prd17b-default-route".to_owned(),
        payload_hash: digest("a"),
        receipt_payload_hash: digest("a"),
        authority_did: "did:exo:production-finality-authority".to_owned(),
        authority_signature: authority_signature(),
        approved_at: "2026-06-20T00:00:00Z".to_owned(),
    }
}

fn packet_acceptance_evidence() -> ContextPacketAcceptanceEvidence {
    ContextPacketAcceptanceEvidence {
        production_default_route_approval_ref: "external-production-approval:context-packet-prd17b"
            .to_owned(),
        packet_quality_review_ref: "external-packet-quality-review:prd17b".to_owned(),
        finality_ref: "external-finality:context-packet-prd17b".to_owned(),
        tenant_id: "dag_db-local".to_owned(),
        memory_namespace: "project_memory_v3".to_owned(),
        actor_id: "did:exo:codex-prd17b".to_owned(),
        route_id: "route-prd17b-default".to_owned(),
        packet_id: "packet-prd17b-default".to_owned(),
        request_id: "request-prd17b-context-packet".to_owned(),
        payload_hash: digest("b"),
        receipt_payload_hash: digest("b"),
        authority_did: "did:exo:production-finality-authority".to_owned(),
        authority_signature: authority_signature(),
        approved_at: "2026-06-20T00:00:01Z".to_owned(),
    }
}

fn placeholder_route_acceptance_evidence() -> DefaultRouteAcceptanceEvidence {
    DefaultRouteAcceptanceEvidence {
        production_default_route_approval_ref: "approval:default-route:prd17b".to_owned(),
        packet_quality_review_ref: "review:packet-quality:prd17b".to_owned(),
        finality_ref: "finality:default-route:prd17b".to_owned(),
        authority_signature: "a".repeat(128),
        ..route_acceptance_evidence()
    }
}

fn placeholder_packet_acceptance_evidence() -> ContextPacketAcceptanceEvidence {
    ContextPacketAcceptanceEvidence {
        production_default_route_approval_ref: "approval:default-route:prd17b".to_owned(),
        packet_quality_review_ref: "review:packet-quality:prd17b".to_owned(),
        finality_ref: "finality:context-packet:prd17b".to_owned(),
        authority_signature: "a".repeat(128),
        ..packet_acceptance_evidence()
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
fn acceptance_evidence_graduates_default_route_and_context_packet() {
    let mut deferred = route();
    deferred.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred.packet_quality_review_status = "operator_deferred".to_owned();

    let accepted_route = accept_default_route_record(
        &deferred,
        &route_acceptance_evidence(),
        "hlc:accepted-route".to_owned(),
    )
    .expect("route approval/finality gates pass");
    let route_report = evaluate_default_route_readiness(&accepted_route).expect("readiness");
    assert_eq!(
        route_report.readiness_status,
        DefaultRuntimeReadinessStatus::Accepted
    );
    assert_eq!(accepted_route.updated_at, "hlc:accepted-route");

    let mut deferred_binding = route_binding();
    deferred_binding.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred_binding.packet_quality_review_status = "operator_deferred".to_owned();
    let deferred_record =
        build_context_packet_record(&deferred_binding, packet_request()).expect("record");
    let deferred_report = build_context_packet_persistence_report(&deferred_record);
    assert!(!deferred_report.accepted);
    assert!(deferred_report.operator_deferred);

    let accepted_record =
        accept_context_packet_record(&deferred_record, &packet_acceptance_evidence())
            .expect("packet approval/finality gates pass");
    let accepted_report = build_context_packet_persistence_report(&accepted_record);
    assert!(accepted_report.accepted);
    assert!(!accepted_report.operator_deferred);
    assert!(
        accepted_record
            .source_proof_refs
            .contains(&"finality:external-finality:context-packet-prd17b".to_owned())
    );
}

#[test]
fn acceptance_evidence_fails_closed_for_missing_finality_and_invalidated_route() {
    let mut deferred = route();
    deferred.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred.packet_quality_review_status = "operator_deferred".to_owned();

    let mut missing_finality = route_acceptance_evidence();
    missing_finality.finality_ref.clear();
    assert!(accept_default_route_record(&deferred, &missing_finality, "hlc:4".to_owned()).is_err());

    let mut invalidated = deferred;
    invalidated.invalidated = true;
    assert!(matches!(
        accept_default_route_record(
            &invalidated,
            &route_acceptance_evidence(),
            "hlc:5".to_owned()
        ),
        Err(DefaultRouteError::AcceptanceGateRejected {
            failure_code: DefaultRetrievalFailureCode::RouteInvalidated
        })
    ));

    let mut deferred_binding = route_binding();
    deferred_binding.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred_binding.packet_quality_review_status = "operator_deferred".to_owned();
    let record = build_context_packet_record(&deferred_binding, packet_request()).expect("record");
    let mut missing_packet_finality = packet_acceptance_evidence();
    missing_packet_finality.finality_ref.clear();
    assert!(accept_context_packet_record(&record, &missing_packet_finality).is_err());
}

#[test]
fn acceptance_evidence_rejects_shaped_placeholder_receipts() {
    let mut deferred = route();
    deferred.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred.packet_quality_review_status = "operator_deferred".to_owned();

    let placeholder_route_evidence = placeholder_route_acceptance_evidence();
    assert!(
        accept_default_route_record(
            &deferred,
            &placeholder_route_evidence,
            "hlc:placeholder".to_owned()
        )
        .is_err(),
        "default-route acceptance must reject caller-shaped placeholder refs"
    );

    let mut deferred_binding = route_binding();
    deferred_binding.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred_binding.packet_quality_review_status = "operator_deferred".to_owned();
    let record = build_context_packet_record(&deferred_binding, packet_request()).expect("record");
    let placeholder_packet_evidence = placeholder_packet_acceptance_evidence();
    assert!(
        accept_context_packet_record(&record, &placeholder_packet_evidence).is_err(),
        "context-packet acceptance must reject caller-shaped placeholder refs"
    );
}

#[test]
fn acceptance_evidence_rejects_scope_hash_actor_and_route_mismatches() {
    let mut deferred = route();
    deferred.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred.packet_quality_review_status = "operator_deferred".to_owned();

    let route_mutations: [fn(&mut DefaultRouteAcceptanceEvidence); 6] = [
        |evidence: &mut DefaultRouteAcceptanceEvidence| evidence.tenant_id = "tenant-b".to_owned(),
        |evidence: &mut DefaultRouteAcceptanceEvidence| {
            evidence.memory_namespace = "namespace-b".to_owned();
        },
        |evidence: &mut DefaultRouteAcceptanceEvidence| {
            evidence.receipt_payload_hash = digest("c");
        },
        |evidence: &mut DefaultRouteAcceptanceEvidence| {
            evidence.actor_id = evidence.authority_did.clone();
        },
        |evidence: &mut DefaultRouteAcceptanceEvidence| {
            evidence.route_id = "route-other".to_owned();
        },
        |evidence: &mut DefaultRouteAcceptanceEvidence| evidence.request_id.clear(),
    ];
    for mutate in route_mutations {
        let mut evidence = route_acceptance_evidence();
        mutate(&mut evidence);
        assert!(
            accept_default_route_record(&deferred, &evidence, "hlc:mismatch".to_owned()).is_err()
        );
    }

    let mut deferred_binding = route_binding();
    deferred_binding.production_default_route_approval_status = "operator_deferred".to_owned();
    deferred_binding.packet_quality_review_status = "operator_deferred".to_owned();
    let record = build_context_packet_record(&deferred_binding, packet_request()).expect("record");
    let packet_mutations: [fn(&mut ContextPacketAcceptanceEvidence); 7] = [
        |evidence: &mut ContextPacketAcceptanceEvidence| evidence.tenant_id = "tenant-b".to_owned(),
        |evidence: &mut ContextPacketAcceptanceEvidence| {
            evidence.memory_namespace = "namespace-b".to_owned();
        },
        |evidence: &mut ContextPacketAcceptanceEvidence| {
            evidence.receipt_payload_hash = digest("d");
        },
        |evidence: &mut ContextPacketAcceptanceEvidence| {
            evidence.actor_id = evidence.authority_did.clone();
        },
        |evidence: &mut ContextPacketAcceptanceEvidence| {
            evidence.route_id = "route-other".to_owned();
        },
        |evidence: &mut ContextPacketAcceptanceEvidence| {
            evidence.packet_id = "packet-other".to_owned();
        },
        |evidence: &mut ContextPacketAcceptanceEvidence| evidence.request_id.clear(),
    ];
    for mutate in packet_mutations {
        let mut evidence = packet_acceptance_evidence();
        mutate(&mut evidence);
        assert!(accept_context_packet_record(&record, &evidence).is_err());
    }
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
