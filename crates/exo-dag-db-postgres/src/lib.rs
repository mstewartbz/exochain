//! PostgreSQL persistence adapters for DAG DB.
//!
//! Facade removal is complete; this crate owns SQLx-backed migrations, import,
//! export, writeback, context selection, idempotency, outbox, and persistence
//! adapters. Downstream compatibility uses explicit bridges such as
//! `exo_api::dagdb`.

#![cfg_attr(test, allow(clippy::expect_used, clippy::unwrap_used))]

#[cfg(feature = "postgres")]
pub use exo_dag_db_core::hash::{
    AgentMemorySafetyScoreIdMaterial, CatalogEntryIdMaterial, ContextPacketIdMaterial,
    CouncilDecisionIdMaterial, InboundAgentCredentialIdMaterial, ParentLink, ReceiptHashMaterial,
    ReceiptMemoryObjectIdMaterial, RequestHashMaterial, RouteIdMaterial,
    ValidationReportIdMaterial,
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_core::{error, hash, metadata, similarity, tenant};
#[cfg(feature = "postgres")]
pub use exo_dag_db_domain::{
    context, context_packet_persistence, continuation_packet, continuation_persistence,
    default_route, export_finality, graph, lifecycle_action, model, placement, route_invalidation,
    scoring, state,
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_domain::{
    context::{ContextPacketDomainInput, build_context_packet},
    context_packet_persistence::{
        CONTEXT_PACKET_RECORD_SCHEMA_VERSION, ContextPacketRecord, DefaultContextQuality,
        PacketFreshnessStatus, PacketPersistenceStatus, PacketValidationStatus,
        canonical_idempotency_key,
    },
    export_finality::ExportFinalityError,
    graph::{MemoryGraphEdge, MemoryGraphNode, PlacementTraceStep, required_placement_steps},
    lifecycle_action::{
        LifecycleAction, LifecycleActionError, LifecycleActionLedger, LifecycleActionType,
        LifecycleApplyResult, LifecycleEvidenceRef, LifecycleMemoryRef, LifecycleRollbackRef,
        LifecycleTerminalState, PRD17_LIFECYCLE_ACTION_SCHEMA,
        PRD17_LIFECYCLE_MUTATION_REPORT_SCHEMA, ProductionLifecycleApproval,
    },
    model::{
        AgentMemorySafetyScore, CatalogEntry, ContextPacket, CouncilDecision, DagDbAuthorizedScope,
        InboundAgentCredential, MemoryCandidateEmitter, MemoryCandidateValidationError,
        OutputObserver, ReceiptMemoryObject, RouteMemoryReceipt, TaskAgentWritebackHint,
        ValidationReport,
    },
    placement::{
        GraphOrganizer, MemoryPlacementController, MemoryPlacementInput, PlacementExistingMemory,
    },
    scoring::{DomainError, DomainResult},
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_exchange::{kg_export, kg_import, kg_writeback, kg_writeback_hygiene};
#[cfg(feature = "postgres")]
pub use exo_dag_db_exchange::{
    kg_export::{
        KG_EXPORT_DATABASE_URL_ENV, KG_EXPORT_FINALITY_OUTBOX_ROUTE_NAME,
        KG_EXPORT_FINALITY_OUTBOX_SUMMARY_SCHEMA, KG_EXPORT_JSON_PATH, KG_EXPORT_MD_PATH,
        KG_EXPORT_PERSISTED_ROUTE_NAME, KG_EXPORT_PERSISTED_SUMMARY_SCHEMA,
        KG_EXPORT_PERSISTENCE_VERIFICATION_SCHEMA, KG_EXPORT_REPORT_ROUTE_NAME,
        KG_EXPORT_TARGET_DIR, KG_PORTABLE_EXPORT_SCHEMA, KG_PORTABLE_EXPORT_SUMMARY_SCHEMA,
        KgExportAcceptance, KgExportArtifactSet, KgExportBuildInput, KgExportError,
        KgExportFinalityOutboxAdvisoryDiagnostics, KgExportFinalityOutboxChallengeDiagnostics,
        KgExportFinalityOutboxDiagnostics, KgExportFinalityOutboxEvidenceDiagnostics,
        KgExportFinalityOutboxReceiptDiagnostics, KgExportFinalityOutboxRequest,
        KgExportFinalityOutboxRowDiagnostics, KgExportFinalityOutboxSummary, KgExportHashes,
        KgExportHlc, KgExportPersistedAdvisoryDiagnostics, KgExportPersistedChallengeDiagnostics,
        KgExportPersistedDiagnostics, KgExportPersistedEvidenceDiagnostics,
        KgExportPersistedIdempotencyDiagnostics, KgExportPersistedReceiptDiagnostics,
        KgExportPersistedRowCounts, KgExportPersistedSectionDiagnostics, KgExportPersistedSummary,
        KgExportPersistenceVerificationSummary, KgExportRecord, KgExportScope,
        KgExportVerification, KgPortableExport, build_portable_export, kg_export_markdown_summary,
        parse_portable_export_json, reject_forbidden_export_json, reject_forbidden_string,
        validate_portable_export_for_persistence, write_default_kg_export_artifacts,
        write_kg_export_artifacts,
    },
    kg_import::{
        KG_IMPORT_CANDIDATES_SCHEMA, KG_IMPORT_DATABASE_URL_ENV, KG_IMPORT_DRY_RUN_REPORT_SCHEMA,
        KG_IMPORT_PERSISTED_ROUTE_NAME, KG_IMPORT_PERSISTED_SUMMARY_SCHEMA, KgImportDryRunReport,
        KgImportError, KgImportPersistedSummary, hash_from_hex, required_trace, stable_hash,
    },
    kg_writeback::{
        KG_WRITEBACK_DATABASE_URL_ENV, KG_WRITEBACK_DRY_RUN_JSON_PATH,
        KG_WRITEBACK_DRY_RUN_MD_PATH, KG_WRITEBACK_DRY_RUN_REPORT_SCHEMA,
        KG_WRITEBACK_DRY_RUN_ROUTE_NAME, KG_WRITEBACK_DRY_RUN_SUMMARY_SCHEMA,
        KG_WRITEBACK_DRY_RUN_TARGET_DIR, KG_WRITEBACK_PERSISTED_ROUTE_NAME,
        KG_WRITEBACK_PERSISTED_SUMMARY_SCHEMA, KgAgentWritebackHint, KgWritebackArtifactSet,
        KgWritebackDryRunReport, KgWritebackError, KgWritebackEvidenceBinding,
        KgWritebackExistingMemory, KgWritebackLayeredWriteback,
        KgWritebackPersistedLayerDiagnostics, KgWritebackPersistedSummary,
        KgWritebackProposalRequest, build_writeback_dry_run_report,
        parse_agent_writeback_hint_json, write_default_writeback_dry_run_artifacts,
        write_writeback_dry_run_artifacts, writeback_markdown_summary,
    },
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_graph::{
    layer_creation_policy, layered_graph, layered_hygiene, layered_placement,
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_graph::{
    layer_creation_policy::{
        LAYER_CREATION_MAX_DEPTH, LAYER_CREATION_POLICY_SCHEMA_VERSION,
        LAYER_CREATION_POLICY_VERSION,
    },
    layered_graph::{
        LAYERED_GRAPH_INVARIANT_REPORT_SCHEMA_VERSION, LayeredGraphInvariantError,
        LayeredGraphInvariantFailure, LayeredGraphInvariantReport, LayeredGraphLayer,
        LayeredGraphLayerEdge, LayeredGraphLayerEdgeKind, LayeredGraphLayerKind,
        LayeredGraphMembership, LayeredGraphMembershipRole, LayeredGraphNodeRef,
        LayeredGraphValidationStatus, build_layered_graph_invariant_report,
        validate_layered_graph_invariants,
    },
    layered_hygiene::{
        LayerHygieneEdgeState, LayerHygienePlan, LayerHygienePolicy,
        build_layered_retrieval_hygiene_report,
    },
    layered_placement::{
        LAYER_PLACEMENT_KNOWLEDGE_GRAPH_PATH, LAYER_PLACEMENT_MAX_DEPTH,
        LAYER_PLACEMENT_REPOSITORY_PATH, LAYER_PLACEMENT_ROOT_PATH, LayerPlacementError,
        LayerPlacementRequest, LayerPlacementSelection, LayerPlacementSourceKind,
        deterministic_layer_edge_id, deterministic_layer_id, deterministic_layer_membership_id,
        select_target_layer,
    },
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_retrieval::context_packet_output::build_graph_context_packet;
#[cfg(feature = "postgres")]
pub use exo_dag_db_retrieval::{
    context_packet_output::{
        GRAPH_CONTEXT_PACKET_SCHEMA_VERSION, LAYERED_CONTEXT_PACKET_OUTPUT_SCHEMA_VERSION,
        LayeredContextPacketAdditions, LayeredContextPacketBudgetReport,
        LayeredContextPacketHygieneReport, LayeredContextPacketOutput,
        LayeredContextPacketRollupSummary, LayeredContextPacketSelectedLayer,
        LayeredContextPacketSelectedLayerEdge, build_layered_context_packet_output,
    },
    graph_context_selection::{
        GraphContextMemoryCandidate, GraphContextSelectionState, MAX_FAMILY_REF_SHARE,
        MAX_ROLLUP_SCORE_BP, MAX_SELECTED_GRAPH_EDGES_PER_PACKET, ROLLUP_SCORING_SHARE_BP,
        TASK_BUDGET_CAP_TOKENS, TASK_BUDGET_FLOOR_TOKENS, TaskBudgetClass,
        classify_task_budget_class, rollup_score_for_root, task_budget_tokens,
    },
    kg_catalog_router::{
        KG_CATALOG_ROUTER_PREVIEW_SCHEMA, KgCatalogPathCandidate, KgCatalogRouterBoundaries,
        KgCatalogRouterBoundaryStatus, KgCatalogRouterEdgeActionClassification,
        KgCatalogRouterError, KgCatalogRouterGraphEdgeRef, KgCatalogRouterMemoryRef,
        KgCatalogRouterOmittedRef, KgCatalogRouterPacketMetrics, KgCatalogRouterPreview,
        KgCatalogRouterScoreComponent, KgCatalogRouterSubgraphRecommendationKind,
        KgCatalogRouterTaskInput, KgSelectedCatalogRoute, KgSubgraphDelegationRecommendation,
    },
    kg_retrieval::{
        KG_CONTEXT_PACKET_PREVIEW_SCHEMA, KG_RETRIEVAL_DATABASE_URL_ENV,
        KG_RETRIEVAL_DEFAULT_MAX_LAYER_DEPTH, KG_RETRIEVAL_DEFAULT_MAX_LAYER_EDGES,
        KG_RETRIEVAL_DEFAULT_MAX_LAYERS_SELECTED, KG_RETRIEVAL_DEFAULT_MAX_MEMORY_REFS,
        KG_RETRIEVAL_DEFAULT_MAX_NODES_PER_LAYER, KG_RETRIEVAL_PREVIEW_ROUTE_NAME,
        KgCitationHandle, KgContextPacketPreview, KgGraphEdgeRef, KgLayerBudgetReport,
        KgLayerCandidate, KgLayerEdgeCandidate, KgLayerEdgeRef, KgLayeredRetrievalBudgetReport,
        KgLayeredRetrievalBudgets, KgLayeredRetrievalRequest, KgLayeredRetrievalSelection,
        KgMemoryRef, KgRetrievalDiagnostics, KgRetrievalError, KgRetrievalRequest,
        KgRollupSummaryRef, KgSelectedLayer, KgSelectedLayerEdge, KgSelectedLayerRef,
        KgValidationSummary, select_layered_retrieval_candidates,
    },
    layered_drilldown::{
        LAYERED_DRILLDOWN_MAX_CHILD_REFS_PER_ROOT, LAYERED_DRILLDOWN_MAX_RESERVE_BP,
        LAYERED_DRILLDOWN_ROOT_FLAG_PREFIX, LAYERED_DRILLDOWN_SELECTION_REASON,
        drilldown_effective_max_depth, drilldown_reserved_breadth_budget, drilldown_root_flag,
        layered_drilldown_active,
    },
    query::{
        GraphQueryService, GraphQueryState, build_context_packet_graph,
        emit_route_invalidation_receipt, graph_route_planner_order,
    },
    views::{
        GraphViewError, GraphViewSource, build_graph_view, regenerate_graph_view_if_stale,
        topological_order, transitive_reduction,
    },
};
#[cfg(feature = "postgres")]
pub use exo_dag_db_retrieval::{
    graph_context_selection, kg_catalog_router, kg_retrieval, layered_drilldown,
};
#[cfg(feature = "postgres")]
pub use graph_context_selection::select_graph_context;
#[cfg(feature = "postgres")]
pub use persistent_context::{
    PersistentGraphContextPacket, PersistentGraphContextSelection,
    build_persistent_graph_context_packet,
    build_persistent_graph_context_packet_with_layered_drilldown,
    build_persistent_graph_context_selection,
    build_persistent_graph_context_selection_with_layered_drilldown,
};
#[cfg(feature = "postgres")]
pub use postgres::kg_catalog_router::{
    KgCatalogRouterPostgresError, build_kg_catalog_router_preview,
    build_kg_catalog_router_preview_from_database_url, build_kg_catalog_router_preview_from_env,
};
#[cfg(feature = "postgres")]
pub use postgres::kg_context_selection_write::{
    CONTEXT_SELECTION_PACKET_RECEIPT_ROUTE_NAME, CONTEXT_SELECTION_USAGE_EVENT_ROUTE_NAME,
    DbWriteSummary, persist_context_packet_receipt_to_db, persist_usage_event_to_db,
};

#[cfg(feature = "postgres")]
pub mod idempotency;
#[cfg(feature = "postgres")]
pub mod outbox;
#[cfg(feature = "postgres")]
pub mod persistent_context;
#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
pub mod receipt;
