//! Compatibility re-export for DAG DB API DTOs.
//!
//! `exo-dag-db-api` owns these wire shapes. `exo-api::dagdb` remains a stable
//! compatibility path for gateway, node, SDK, and external callers.

pub use exo_dag_db_api::{
    CanonicalizationDecision, CanonicalizationDecisionKind, CatalogEntryResponse, ConsentPurpose,
    ContextPacketLayerBudgetReport, ContextPacketLayerEdgeRef, ContextPacketLayerRef,
    ContextPacketMemoryRef, CouncilDecisionStatus, CouncilReviewStatus, CredentialStatus,
    DAGDB_CATALOG_LOOKUP_RESPONSE_SCHEMA_VERSION, DAGDB_CONTEXT_PACKET_RESPONSE_SCHEMA_VERSION,
    DAGDB_COUNCIL_DECISION_RESPONSE_SCHEMA_VERSION, DAGDB_EXPORT_RESPONSE_SCHEMA_VERSION,
    DAGDB_IMPORT_RESPONSE_SCHEMA_VERSION, DAGDB_INTAKE_RESPONSE_SCHEMA_VERSION,
    DAGDB_RECEIPT_LOOKUP_RESPONSE_SCHEMA_VERSION, DAGDB_ROUTE_LOOKUP_RESPONSE_SCHEMA_VERSION,
    DAGDB_ROUTE_RESPONSE_SCHEMA_VERSION, DAGDB_TRUST_CHECK_RESPONSE_SCHEMA_VERSION,
    DAGDB_VALIDATE_RESPONSE_SCHEMA_VERSION, DAGDB_WRITEBACK_RESPONSE_SCHEMA_VERSION,
    DagDbCatalogLookupRequest, DagDbCatalogLookupResponse, DagDbContextPacketBoundaries,
    DagDbContextPacketCitationRef, DagDbContextPacketImportTrackingStatus,
    DagDbContextPacketMetrics, DagDbContextPacketRequest, DagDbContextPacketResponse,
    DagDbCouncilDecisionRequest, DagDbCouncilDecisionResponse, DagDbErrorEnvelope,
    DagDbExportRequest, DagDbExportResponse, DagDbGraphContextPacket,
    DagDbGraphContextPacketBuildRequest, DagDbGraphContextSelectionRequest,
    DagDbGraphContextSelectionResponse, DagDbGraphContextSelectionStatus,
    DagDbGraphSelectionTraceStep, DagDbImportRequest, DagDbImportResponse, DagDbIntakeRequest,
    DagDbIntakeResponse, DagDbOmittedContextRef, DagDbReceiptLookupRequest,
    DagDbReceiptLookupResponse, DagDbRouteLookupRequest, DagDbRouteLookupResponse,
    DagDbRouteRequest, DagDbRouteResponse, DagDbSelectedContextRef, DagDbSelectedGraphEdgeRef,
    DagDbTrustCheckRequest, DagDbTrustCheckResponse, DagDbValidateRequest, DagDbValidateResponse,
    DagDbWritebackRequest, DagDbWritebackResponse, DagFinalityStatus, DecisionSource, GraphEdgeRef,
    GraphView, GraphViewType, MemoryCandidate, MemoryCandidateKind, MemoryCandidateUse,
    MemoryEdgeKind, MemoryEdgeType, MemoryGraphStyle, MemoryNodeKind, MemoryNodeType, MemoryStatus,
    PlacementResult, ReceiptEventType, RedactionCode, RiskClass, RouteInvalidationReceipt,
    RouteInvalidationStatus, RouteInvalidationTrigger, RouteStatus, SafeMetadata,
    SafeMetadataDecision, SimilarityResult, SimilarityType, SourceType, SubjectKind,
    ValidationDecision, ValidationStatus,
};
