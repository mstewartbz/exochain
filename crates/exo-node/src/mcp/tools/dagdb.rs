//! DAG DB MCP tools — the canonical agent-facing surface.
//!
//! These four tools (`dagdb_get_context_packet`, `dagdb_submit_writeback`,
//! `dagdb_import`, `dagdb_export`) are the unified Rust home for the DAG DB MCP
//! surface (GAP-012 P1-C). They supersede the legacy, unversioned,
//! markdown-returning sidecar surface while keeping this upstream package
//! self-contained.
//!
//! Each tool's `input_schema` is BOUND to the versioned `exo-api` DAG DB
//! request DTOs: the schema's `required`/optional property sets mirror the DTO
//! fields, and a schema-drift test (`tests::schemas_stay_bound_to_exo_api_dtos`)
//! validates the shared `exo-dag-db` JSON fixtures against the compiled schemas
//! and round-trips them through the DTOs, so a DTO field add/remove/rename
//! fails the test instead of silently drifting.
//!
//! ## Opt-in adapter boundary (T6) and the proxy
//!
//! dag-db is an OPT-IN adapter the default node does NOT serve. When no
//! operator has configured a DAG DB gateway (the default), every tool FAILS
//! CLOSED with a structured `dagdb_adapter_unconfigured` result — it never
//! fabricates import/export/packet/writeback success. The opt-in path lives
//! behind the `dagdb-gateway-proxy` feature, which pulls the P1-B SDK
//! (`exochain-sdk` `DagDbHttpClient`, `http-client` feature) so the default
//! lean node stays free of the async HTTP stack.
//!
//! ## DEFERRED: live gateway proxy wiring
//!
//! The actual proxy call (build a typed `DagDb*Request`, invoke
//! `DagDbHttpClient`, map the typed `DagDbClientError` to a structured MCP
//! error) is NOT yet wired. The MCP dispatch chain
//! (`handler::dispatch` -> `handle_tools_call` -> `ToolRegistry::execute` ->
//! `execute_*`) is fully synchronous, while `DagDbHttpClient` is async; wiring
//! the proxy requires either making the dispatch chain async or blocking on a
//! runtime handle inside the sync handler, plus threading the gateway auth
//! material (`DagDbAuthConfig`) through `NodeContext`. That is a separate,
//! larger refactor. This ticket lands: all 4 DTO-bound tools, the structured
//! fail-closed result, the schema-drift test, the opt-in feature boundary, and
//! the legacy sidecar demotion. When the proxy is wired, `execute_*` will gain
//! an explicit configured-gateway path and call the SDK; until then it always
//! returns the unconfigured result.

use serde_json::{Value, json};

use crate::mcp::{
    context::NodeContext,
    protocol::{ToolDefinition, ToolResult},
};

const MAX_SAFE_ID_BYTES: usize = 128;
const SAFE_ID_PATTERN: &str = "^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$";
const DID_PATTERN: &str = "^did:[A-Za-z0-9][A-Za-z0-9._:-]{0,123}$";
const SAFE_PATH_PATTERN: &str =
    "^(?!/)(?!~)(?!.*\\\\)(?!.*(^|/)\\.\\.?(/|$))[A-Za-z0-9][A-Za-z0-9._/:-]{0,255}$";
const HASH256_PATTERN: &str = "^[0-9a-f]{64}$";
const DAGDB_ADAPTER_UNCONFIGURED: &str = "dagdb_adapter_unconfigured";
const DAGDB_GET_CONTEXT_PACKET_TOOL: &str = "dagdb_get_context_packet";
const DAGDB_SUBMIT_WRITEBACK_TOOL: &str = "dagdb_submit_writeback";
const DAGDB_IMPORT_TOOL: &str = "dagdb_import";
const DAGDB_EXPORT_TOOL: &str = "dagdb_export";
const MAX_ID_ARRAY_ITEMS: usize = 256;
const MAX_TOKEN_BUDGET: u64 = 1_000_000;
const KG_IMPORT_REPORT_SCHEMA: &str = "dagdb_kg_dry_run_import_report_v1";
const KG_IMPORT_CANDIDATES_SCHEMA: &str = "dagdb_markdown_kg_import_candidates_v1";
const ECHOED_STRING_FIELDS: &[(&str, &str)] = &[
    ("idempotency_key", "operation_id"),
    ("tenant_id", "tenant_id"),
    ("namespace", "namespace"),
    ("db_set_version", "db_set_version"),
];
const FORBIDDEN_ECHO_FRAGMENTS: &[&str] = &[
    "sk-proj-",
    "password",
    "token",
    "ghp_",
    "github_pat_",
    "xoxb-",
];

const NON_CLAIMS: &[&str] = &[
    "no_runtime_dagdb_operation_was_performed",
    "no_persistence_receipt_was_created",
    "no_export_artifact_was_created",
    "dagdb_adapter_is_opt_in_and_not_configured_on_this_node",
];

fn safe_string_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "minLength": 1,
        "maxLength": MAX_SAFE_ID_BYTES,
        "pattern": SAFE_ID_PATTERN,
        "description": description,
    })
}

fn optional_safe_string_schema(description: &str) -> Value {
    json!({
        "anyOf": [
            {
                "type": "string",
                "minLength": 1,
                "maxLength": MAX_SAFE_ID_BYTES,
                "pattern": SAFE_ID_PATTERN,
            },
            {
                "type": "null",
            }
        ],
        "description": description,
    })
}

fn did_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "minLength": 5,
        "maxLength": MAX_SAFE_ID_BYTES,
        "pattern": DID_PATTERN,
        "description": description,
    })
}

fn safe_path_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "minLength": 1,
        "maxLength": 256,
        "pattern": SAFE_PATH_PATTERN,
        "description": description,
    })
}

fn hash_schema(description: &str) -> Value {
    json!({
        "type": "string",
        "pattern": HASH256_PATTERN,
        "description": description,
    })
}

fn token_budget_schema(description: &str) -> Value {
    json!({
        "type": "integer",
        "minimum": 0,
        "maximum": MAX_TOKEN_BUDGET,
        "description": description,
    })
}

fn optional_bool_schema(description: &str) -> Value {
    json!({
        "anyOf": [{"type": "boolean"}, {"type": "null"}],
        "description": description,
    })
}

fn optional_token_budget_schema(description: &str) -> Value {
    json!({
        "anyOf": [{"type": "integer", "minimum": 0, "maximum": MAX_TOKEN_BUDGET}, {"type": "null"}],
        "description": description,
    })
}

fn optional_safe_string_array_schema(description: &str) -> Value {
    json!({
        "anyOf": [
            {
                "type": "array",
                "maxItems": MAX_ID_ARRAY_ITEMS,
                "uniqueItems": true,
                "items": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": MAX_SAFE_ID_BYTES,
                    "pattern": SAFE_ID_PATTERN,
                },
            },
            {"type": "null"}
        ],
        "description": description,
    })
}

fn optional_hash_array_schema(description: &str) -> Value {
    json!({
        "anyOf": [
            {
                "type": "array",
                "maxItems": MAX_ID_ARRAY_ITEMS,
                "uniqueItems": true,
                "items": hash_schema("64-character lowercase hex hash."),
            },
            {"type": "null"}
        ],
        "description": description,
    })
}

fn optional_safe_text_schema(description: &str) -> Value {
    json!({
        "anyOf": [{"type": "string", "maxLength": 4096}, {"type": "null"}],
        "description": description,
    })
}

fn common_properties() -> serde_json::Map<String, Value> {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "idempotency_key".to_owned(),
        safe_string_schema("Caller-supplied idempotency key for the DAG DB operation."),
    );
    properties.insert(
        "tenant_id".to_owned(),
        safe_string_schema("Tenant boundary for the requested DAG DB operation."),
    );
    properties.insert(
        "namespace".to_owned(),
        safe_string_schema("Namespace boundary for the requested DAG DB operation."),
    );
    properties.insert(
        "db_set_version".to_owned(),
        safe_string_schema("DAG DB set version for the requested operation."),
    );
    properties
}

fn safe_string_array_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "maxItems": MAX_ID_ARRAY_ITEMS,
        "uniqueItems": true,
        "items": {
            "type": "string",
            "minLength": 1,
            "maxLength": MAX_SAFE_ID_BYTES,
            "pattern": SAFE_ID_PATTERN,
        },
        "description": description,
    })
}

fn hash_array_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "maxItems": MAX_ID_ARRAY_ITEMS,
        "uniqueItems": true,
        "items": hash_schema("64-character lowercase hex hash."),
        "description": description,
    })
}

fn empty_array_schema(description: &str) -> Value {
    json!({
        "type": "array",
        "maxItems": 0,
        "description": description,
    })
}

fn empty_object_schema(description: &str) -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "description": description,
    })
}

fn import_report_schema() -> Value {
    json!({
        "type": "object",
        "description": "Strict REST-compatible DAG DB dry-run import report envelope accepted by the MCP runtime surface. This pass accepts the gateway/SDK fixture shape with empty proposed record arrays and rejects digest-only summaries.",
        "properties": {
            "schema_version": {
                "type": "string",
                "enum": [KG_IMPORT_REPORT_SCHEMA],
            },
            "source_candidates_schema_version": {
                "type": "string",
                "enum": [KG_IMPORT_CANDIDATES_SCHEMA],
            },
            "graph_root": safe_path_schema("Repository-relative knowledge graph root."),
            "tenant_id": safe_string_schema("Tenant boundary embedded in the dry-run import report."),
            "namespace": safe_string_schema("Namespace boundary embedded in the dry-run import report."),
            "actor_did": did_schema("DID that produced the dry-run import report."),
            "batch_id": hash_schema("64-character lowercase hex dry-run import batch ID."),
            "dry_run_only": {
                "type": "boolean",
                "enum": [true],
            },
            "postgres_writes": {
                "type": "boolean",
                "enum": [false],
            },
            "raw_markdown_included": {
                "type": "boolean",
                "enum": [false],
            },
            "proposed_memory_records": empty_array_schema("Proposed memory records. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_catalog_entries": empty_array_schema("Proposed catalog entries. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_graph_nodes": empty_array_schema("Proposed graph nodes. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_graph_edges": empty_array_schema("Proposed graph edges. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_required_edges": empty_array_schema("Proposed required edges. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_placement_decisions": empty_array_schema("Proposed placement decisions. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_receipt_intents": empty_array_schema("Proposed receipt intents. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_validation_reports": empty_array_schema("Proposed validation reports. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_governance_reviews": empty_array_schema("Proposed governance reviews. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_graph_view_refreshes": empty_array_schema("Proposed graph view refreshes. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_route_invalidations": empty_array_schema("Proposed route invalidations. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "proposed_subdag_boundaries": empty_array_schema("Proposed subdag boundaries. Empty fixture arrays are accepted by this bounded MCP schema pass."),
            "rollback_plan": empty_object_schema("Dry-run rollback plan metadata."),
            "placement_governance_summary": empty_object_schema("Dry-run placement governance metadata."),
            "review_items": empty_array_schema("Dry-run review items."),
            "warnings": empty_array_schema("Dry-run warnings."),
        },
        "required": [
            "schema_version",
            "source_candidates_schema_version",
            "graph_root",
            "tenant_id",
            "namespace",
            "actor_did",
            "batch_id",
            "dry_run_only",
            "postgres_writes",
            "raw_markdown_included",
            "proposed_memory_records",
            "proposed_catalog_entries",
            "proposed_graph_nodes",
            "proposed_graph_edges",
            "proposed_placement_decisions",
            "proposed_receipt_intents",
            "proposed_validation_reports"
        ],
        "additionalProperties": false,
    })
}

fn echoed_field_with_forbidden_fragment(params: &Value) -> Option<&'static str> {
    for (request_field, response_field) in ECHOED_STRING_FIELDS {
        let Some(value) = params.get(*request_field).and_then(Value::as_str) else {
            continue;
        };
        let normalized = value.to_ascii_lowercase();
        if FORBIDDEN_ECHO_FRAGMENTS
            .iter()
            .any(|fragment| normalized.contains(fragment))
        {
            return Some(*response_field);
        }
    }

    None
}

/// Structured fail-closed result for the opt-in DAG DB adapter.
///
/// Returned whenever no DAG DB gateway is configured (the default, and the
/// only path until the proxy is wired). Never claims any runtime effect.
fn mcp_json_error(message: &str, fields: Value) -> ToolResult {
    let mut body = match fields {
        Value::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    body.insert("message".to_owned(), json!(message));
    ToolResult::error(Value::Object(body).to_string())
}

fn adapter_unconfigured_response(tool_name: &str, params: &Value) -> ToolResult {
    if let Some(field) = echoed_field_with_forbidden_fragment(params) {
        return mcp_json_error(
            "DAG DB request rejected before unsafe echo.",
            json!({
                "tool_status": "rejected_unsafe_echo_field",
                "tool": tool_name,
                "field": field,
            }),
        );
    }

    tracing::warn!(
        tool = %tool_name,
        "refusing DAG DB MCP call: no DAG DB gateway is configured (opt-in adapter unconfigured)"
    );

    mcp_json_error(
        "DAG DB adapter is not configured on this node; no DAG DB operation was performed.",
        json!({
            "tool_status": DAGDB_ADAPTER_UNCONFIGURED,
            "tool": tool_name,
            "operation_id": params.get("idempotency_key").and_then(Value::as_str),
            "tenant_id": params.get("tenant_id").and_then(Value::as_str),
            "namespace": params.get("namespace").and_then(Value::as_str),
            "db_set_version": params.get("db_set_version").and_then(Value::as_str),
            "non_claims": NON_CLAIMS,
        }),
    )
}

/// Tool definition for `dagdb_get_context_packet`.
///
/// Input schema is bound to `exo_api::dagdb::DagDbContextPacketRequest`.
#[must_use]
pub fn get_context_packet_definition() -> ToolDefinition {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "tenant_id".to_owned(),
        safe_string_schema("Tenant boundary for the context-packet request."),
    );
    properties.insert(
        "namespace".to_owned(),
        safe_string_schema("Namespace boundary for the context-packet request."),
    );
    properties.insert(
        "idempotency_key".to_owned(),
        safe_string_schema("Caller-supplied idempotency key for the context-packet request."),
    );
    properties.insert(
        "request_id".to_owned(),
        safe_string_schema("Caller-supplied context-packet request ID."),
    );
    properties.insert(
        "route_id".to_owned(),
        hash_schema("64-character lowercase hex route ID the packet is scoped to."),
    );
    properties.insert(
        "task_hash".to_owned(),
        hash_schema("64-character lowercase hex digest of the task the packet optimizes for."),
    );
    properties.insert(
        "requesting_agent_did".to_owned(),
        did_schema("DID of the agent requesting the context packet."),
    );
    properties.insert(
        "token_budget".to_owned(),
        token_budget_schema("Token budget the packet must fit within."),
    );
    properties.insert(
        "force_revalidate".to_owned(),
        optional_bool_schema("Force re-validation of cached packet contents."),
    );
    properties.insert(
        "max_memory_refs".to_owned(),
        optional_token_budget_schema("Maximum number of memory refs to return."),
    );
    properties.insert(
        "task".to_owned(),
        optional_safe_text_schema("Optional free-text task description for packet selection."),
    );
    properties.insert(
        "layered_mode".to_owned(),
        optional_safe_string_schema("Optional layered-context mode (`off`/`auto`/`required`)."),
    );
    properties.insert(
        "max_layer_depth".to_owned(),
        optional_token_budget_schema("Optional maximum layer traversal depth."),
    );
    properties.insert(
        "require_layer_evidence".to_owned(),
        optional_bool_schema("Fail closed when requested layer evidence is missing."),
    );
    properties.insert(
        "drilldown_reserve_bp".to_owned(),
        optional_token_budget_schema("Depth-on-demand reserve, in basis points of the budget."),
    );

    ToolDefinition {
        name: DAGDB_GET_CONTEXT_PACKET_TOOL.to_owned(),
        description: "Retrieve a graph-routed DAG DB context packet for a task through the runtime MCP surface. dag-db is an opt-in adapter; when no gateway is configured this node fails closed with a structured `dagdb_adapter_unconfigured` result instead of fabricating a packet.".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": [
                "tenant_id",
                "namespace",
                "idempotency_key",
                "request_id",
                "route_id",
                "task_hash",
                "requesting_agent_did",
                "token_budget"
            ],
            "additionalProperties": false,
        }),
    }
}

/// Tool definition for `dagdb_submit_writeback`.
///
/// Input schema is bound to `exo_api::dagdb::DagDbWritebackRequest`.
#[must_use]
pub fn submit_writeback_definition() -> ToolDefinition {
    let mut properties = serde_json::Map::new();
    properties.insert(
        "tenant_id".to_owned(),
        safe_string_schema("Tenant boundary for the writeback request."),
    );
    properties.insert(
        "namespace".to_owned(),
        safe_string_schema("Namespace boundary for the writeback request."),
    );
    properties.insert(
        "idempotency_key".to_owned(),
        safe_string_schema("Caller-supplied idempotency key for the writeback request."),
    );
    properties.insert(
        "requesting_agent_did".to_owned(),
        did_schema("DID of the agent submitting the writeback."),
    );
    properties.insert(
        "parent_memory_ids".to_owned(),
        hash_array_schema("64-character lowercase hex parent memory IDs from the context packet."),
    );
    properties.insert(
        "answer_hash".to_owned(),
        hash_schema("64-character lowercase hex digest of the completed answer."),
    );
    properties.insert(
        "route_id".to_owned(),
        hash_schema("64-character lowercase hex route ID the writeback belongs to."),
    );
    properties.insert(
        "context_packet_id".to_owned(),
        hash_schema("64-character lowercase hex context packet ID the writeback cites."),
    );
    properties.insert(
        "validation_report_id".to_owned(),
        hash_schema("64-character lowercase hex validation report ID."),
    );
    properties.insert(
        "summary_text".to_owned(),
        optional_safe_text_schema("Optional bounded summary of the completed work."),
    );
    properties.insert(
        "citation_hashes".to_owned(),
        optional_hash_array_schema("Optional 64-character lowercase hex citation hashes."),
    );
    properties.insert(
        "safety_score_id".to_owned(),
        optional_safe_string_schema("Optional safety-score ID."),
    );
    properties.insert(
        "keyword_texts".to_owned(),
        optional_safe_string_array_schema("Optional keyword texts for later recall."),
    );
    properties.insert(
        "knowledge_class".to_owned(),
        optional_safe_string_schema("Optional typed-knowledge class (e.g. `decision`/`finding`)."),
    );
    properties.insert(
        "layered_mode".to_owned(),
        optional_safe_string_schema("Optional layered writeback mode."),
    );
    properties.insert(
        "target_layer_path".to_owned(),
        optional_safe_string_schema("Optional repo-local target layer path."),
    );
    properties.insert(
        "target_layer_depth".to_owned(),
        optional_token_budget_schema("Optional depth of the target layer path."),
    );
    properties.insert(
        "target_layer_reason".to_owned(),
        optional_safe_string_schema("Optional safe reason code for the target layer writeback."),
    );

    ToolDefinition {
        name: DAGDB_SUBMIT_WRITEBACK_TOOL.to_owned(),
        description: "Submit completed-task evidence to the DAG DB writeback endpoint through the runtime MCP surface, with context-packet lineage. dag-db is an opt-in adapter; when no gateway is configured this node fails closed with a structured `dagdb_adapter_unconfigured` result instead of fabricating a writeback receipt.".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": [
                "tenant_id",
                "namespace",
                "idempotency_key",
                "requesting_agent_did",
                "parent_memory_ids",
                "answer_hash",
                "route_id",
                "context_packet_id",
                "validation_report_id"
            ],
            "additionalProperties": false,
        }),
    }
}

/// Tool definition for `dagdb_import`.
///
/// Input schema is bound to `exo_api::dagdb::DagDbImportRequest`.
#[must_use]
pub fn import_definition() -> ToolDefinition {
    let mut properties = common_properties();
    properties.insert(
        "source_hash".to_owned(),
        hash_schema("64-character digest for the approved import source material."),
    );
    properties.insert(
        "requester_did".to_owned(),
        did_schema("DID requesting the import operation."),
    );
    properties.insert("import_report".to_owned(), import_report_schema());

    ToolDefinition {
        name: DAGDB_IMPORT_TOOL.to_owned(),
        description: "Request a governed DAG DB import through the runtime MCP surface. dag-db is an opt-in adapter; when no gateway is configured this node fails closed with a structured `dagdb_adapter_unconfigured` result instead of fabricating persistence.".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": [
                "tenant_id",
                "namespace",
                "idempotency_key",
                "db_set_version",
                "source_hash",
                "requester_did",
                "import_report"
            ],
            "additionalProperties": false,
        }),
    }
}

/// Tool definition for `dagdb_export`.
///
/// Input schema is bound to `exo_api::dagdb::DagDbExportRequest`.
#[must_use]
pub fn export_definition() -> ToolDefinition {
    let mut properties = common_properties();
    properties.insert(
        "requester_did".to_owned(),
        did_schema("DID requesting the export operation."),
    );
    properties.insert(
        "included_memory_ids".to_owned(),
        hash_array_schema("64-character lowercase hex memory IDs included in the export scope."),
    );
    properties.insert(
        "included_graph_styles".to_owned(),
        safe_string_array_schema("Graph styles included in the export scope."),
    );
    properties.insert(
        "included_writeback_idempotency_keys".to_owned(),
        safe_string_array_schema("Writeback idempotency keys included in the export scope."),
    );
    properties.insert(
        "source_commit_or_repo_ref".to_owned(),
        optional_safe_string_schema("Optional commit or repository ref for export provenance."),
    );
    properties.insert(
        "include_preview_context".to_owned(),
        json!({
            "type": "boolean",
            "description": "Whether preview-only context sections are requested.",
        }),
    );

    ToolDefinition {
        name: DAGDB_EXPORT_TOOL.to_owned(),
        description: "Request a governed DAG DB export through the runtime MCP surface. dag-db is an opt-in adapter; when no gateway is configured this node fails closed with a structured `dagdb_adapter_unconfigured` result instead of fabricating export artifacts.".to_owned(),
        input_schema: json!({
            "type": "object",
            "properties": properties,
            "required": [
                "tenant_id",
                "namespace",
                "idempotency_key",
                "db_set_version",
                "requester_did",
                "included_memory_ids",
                "included_graph_styles",
                "included_writeback_idempotency_keys",
                "include_preview_context"
            ],
            "additionalProperties": false,
        }),
    }
}

/// Dispatch a tool through the current fail-closed adapter boundary.
///
/// Until the async proxy and `NodeContext` gateway state are wired (see the
/// module doc's DEFERRED note), this always returns the structured
/// `dagdb_adapter_unconfigured` result.
fn dispatch(tool_name: &str, params: &Value, _context: &NodeContext) -> ToolResult {
    adapter_unconfigured_response(tool_name, params)
}

/// Execute `dagdb_get_context_packet`.
#[must_use]
pub fn execute_get_context_packet(params: &Value, context: &NodeContext) -> ToolResult {
    dispatch(DAGDB_GET_CONTEXT_PACKET_TOOL, params, context)
}

/// Execute `dagdb_submit_writeback`.
#[must_use]
pub fn execute_submit_writeback(params: &Value, context: &NodeContext) -> ToolResult {
    dispatch(DAGDB_SUBMIT_WRITEBACK_TOOL, params, context)
}

/// Execute `dagdb_import`.
#[must_use]
pub fn execute_import(params: &Value, context: &NodeContext) -> ToolResult {
    dispatch(DAGDB_IMPORT_TOOL, params, context)
}

/// Execute `dagdb_export`.
#[must_use]
pub fn execute_export(params: &Value, context: &NodeContext) -> ToolResult {
    dispatch(DAGDB_EXPORT_TOOL, params, context)
}

#[cfg(test)]
mod tests {
    use jsonschema::JSONSchema;
    use serde_json::Value;

    use super::*;

    const FIXTURES: &str =
        include_str!("../../../../exo-dag-db-api/fixtures/json/all_dto_fixtures.json");

    fn fixtures() -> Value {
        serde_json::from_str(FIXTURES).expect("DAG DB fixture set parses")
    }

    fn request_fixture(name: &str) -> Value {
        fixtures()
            .get("requests")
            .and_then(|requests| requests.get(name))
            .unwrap_or_else(|| panic!("missing request fixture {name}"))
            .clone()
    }

    fn valid_import_params() -> Value {
        json!({
            "idempotency_key": "m60-import-001",
            "tenant_id": "dag_db-local",
            "namespace": "dag_db",
            "db_set_version": "project_memory_v3",
            "source_hash": "1111111111111111111111111111111111111111111111111111111111111111",
            "requester_did": "did:exo:requester",
            "import_report": {
                "schema_version": KG_IMPORT_REPORT_SCHEMA,
                "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
                "graph_root": "docs/dagdb/crate-restructure",
                "tenant_id": "dag_db-local",
                "namespace": "dag_db",
                "actor_did": "did:exo:kg-importer",
                "batch_id": "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd",
                "dry_run_only": true,
                "postgres_writes": false,
                "raw_markdown_included": false,
                "proposed_memory_records": [],
                "proposed_catalog_entries": [],
                "proposed_graph_nodes": [],
                "proposed_graph_edges": [],
                "proposed_required_edges": [],
                "proposed_placement_decisions": [],
                "proposed_receipt_intents": [],
                "proposed_validation_reports": [],
                "proposed_governance_reviews": [],
                "proposed_graph_view_refreshes": [],
                "proposed_route_invalidations": [],
                "proposed_subdag_boundaries": [],
                "rollback_plan": {},
                "placement_governance_summary": {},
                "review_items": [],
                "warnings": [],
            },
        })
    }

    fn valid_export_params() -> Value {
        json!({
            "idempotency_key": "m60-export-001",
            "tenant_id": "dag_db-local",
            "namespace": "dag_db",
            "db_set_version": "project_memory_v3",
            "requester_did": "did:exo:requester",
            "included_memory_ids": ["2222222222222222222222222222222222222222222222222222222222222222"],
            "included_graph_styles": ["chronological"],
            "included_writeback_idempotency_keys": ["writeback-001"],
            "include_preview_context": false,
        })
    }

    fn result_json(result: &ToolResult) -> Value {
        serde_json::from_str(result.content[0].text()).expect("tool result text is JSON")
    }

    fn assert_rejected_without_echo(mut params: Value, request_field: &str, unsafe_value: &str) {
        params[request_field] = json!(unsafe_value);

        let result = execute_export(&params, &NodeContext::empty());
        assert!(result.is_error);

        let raw = result_json(&result).to_string().to_ascii_lowercase();
        assert_eq!(
            result_json(&result)["tool_status"],
            "rejected_unsafe_echo_field"
        );
        assert!(
            !raw.contains(&unsafe_value.to_ascii_lowercase()),
            "rejection must not echo unsafe value: {raw}"
        );
    }

    #[test]
    fn import_tool_fails_closed_with_unconfigured_adapter_result() {
        let result = execute_import(&valid_import_params(), &NodeContext::empty());
        assert!(result.is_error);
        assert_eq!(
            result_json(&result)["message"],
            "DAG DB adapter is not configured on this node; no DAG DB operation was performed."
        );
        let body = result_json(&result);
        assert_eq!(body["tool_status"], DAGDB_ADAPTER_UNCONFIGURED);
        assert_eq!(body["tool"], DAGDB_IMPORT_TOOL);
        assert_eq!(body["operation_id"], "m60-import-001");
        assert_eq!(body["tenant_id"], "dag_db-local");
        assert_eq!(body["namespace"], "dag_db");
        assert_eq!(body["db_set_version"], "project_memory_v3");
        assert!(
            body["non_claims"]
                .as_array()
                .expect("non_claims array")
                .contains(&json!("no_runtime_dagdb_operation_was_performed"))
        );

        let raw = body.to_string().to_ascii_lowercase();
        for forbidden in [
            "/users/",
            "source_body",
            "raw_prompt_body",
            "receipt_path",
            "export_artifact_path",
            "sk-proj-",
            "password",
        ] {
            assert!(
                !raw.contains(forbidden),
                "fail-closed response must not leak {forbidden}: {raw}"
            );
        }
    }

    #[test]
    fn export_tool_fails_closed_with_unconfigured_adapter_result() {
        let result = execute_export(&valid_export_params(), &NodeContext::empty());
        assert!(result.is_error);
        let body = result_json(&result);
        assert_eq!(body["tool_status"], DAGDB_ADAPTER_UNCONFIGURED);
        assert_eq!(body["tool"], DAGDB_EXPORT_TOOL);
        assert_eq!(body["operation_id"], "m60-export-001");
        assert!(
            body["non_claims"]
                .as_array()
                .expect("non_claims array")
                .contains(&json!("no_export_artifact_was_created"))
        );
    }

    #[test]
    fn context_packet_tool_fails_closed_with_unconfigured_adapter_result() {
        let result =
            execute_get_context_packet(&request_fixture("context_packet"), &NodeContext::empty());
        assert!(result.is_error);
        let body = result_json(&result);
        assert_eq!(body["tool_status"], DAGDB_ADAPTER_UNCONFIGURED);
        assert_eq!(body["tool"], DAGDB_GET_CONTEXT_PACKET_TOOL);
        assert_eq!(body["operation_id"], "idem-packet-1");
        assert_eq!(body["tenant_id"], "tenant-a");
    }

    #[test]
    fn writeback_tool_fails_closed_with_unconfigured_adapter_result() {
        let result = execute_submit_writeback(&request_fixture("writeback"), &NodeContext::empty());
        assert!(result.is_error);
        let body = result_json(&result);
        assert_eq!(body["tool_status"], DAGDB_ADAPTER_UNCONFIGURED);
        assert_eq!(body["tool"], DAGDB_SUBMIT_WRITEBACK_TOOL);
        assert_eq!(body["operation_id"], "idem-writeback-1");
        assert_eq!(body["tenant_id"], "tenant-a");
    }

    #[test]
    fn echoed_fields_reject_secret_like_fragments_without_echoing_them() {
        for (request_field, unsafe_value) in [
            ("idempotency_key", "sk-proj-abc"),
            ("tenant_id", "password-token"),
            ("namespace", "ghp_common-token-prefix"),
            ("db_set_version", "release-token-v1"),
        ] {
            assert_rejected_without_echo(valid_export_params(), request_field, unsafe_value);
        }
    }

    #[test]
    fn export_schema_accepts_null_source_commit_or_repo_ref() {
        let definition = export_definition();
        let validator =
            JSONSchema::compile(&definition.input_schema).expect("export schema compiles");
        let mut params = valid_export_params();
        params["source_commit_or_repo_ref"] = Value::Null;

        let errors = validator
            .validate(&params)
            .err()
            .map(|errors| errors.map(|err| err.to_string()).collect::<Vec<_>>());
        assert!(
            errors.is_none(),
            "source_commit_or_repo_ref: null should validate, got: {errors:?}"
        );
    }

    #[test]
    fn import_report_schema_rejects_arbitrary_nested_payloads_before_dispatch() {
        let definition = import_definition();
        let validator =
            JSONSchema::compile(&definition.input_schema).expect("import schema compiles");

        for import_report in [
            json!({"raw_source_body": "must not be accepted"}),
            json!({
                "schema_version": KG_IMPORT_REPORT_SCHEMA,
                "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
                "source_path": "/Users/example/private.json",
            }),
            json!({
                "schema_version": KG_IMPORT_REPORT_SCHEMA,
                "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
                "batch_id": "not-a-hash",
            }),
            json!({
                "schema_version": KG_IMPORT_REPORT_SCHEMA,
                "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
                "sk-proj-secret-nested": "sk-proj-secret-value",
            }),
            json!({
                "schema_version": "sk-proj-secret-schema",
                "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
            }),
        ] {
            let mut params = valid_import_params();
            params["import_report"] = import_report;
            assert!(
                validator.validate(&params).is_err(),
                "import_report payload must be rejected before dispatch: {params}"
            );
        }
    }

    #[test]
    fn definitions_are_strict_object_schemas() {
        for definition in [
            get_context_packet_definition(),
            submit_writeback_definition(),
            import_definition(),
            export_definition(),
        ] {
            assert_eq!(definition.input_schema["type"], "object");
            assert_eq!(definition.input_schema["additionalProperties"], false);
            for required in ["idempotency_key", "tenant_id", "namespace"] {
                assert!(
                    definition.input_schema["required"]
                        .as_array()
                        .expect("required array")
                        .contains(&json!(required)),
                    "{} must require {required}",
                    definition.name
                );
            }
        }
    }

    #[test]
    fn import_report_schema_rejects_digest_only_summary() {
        let definition = import_definition();
        let validator =
            JSONSchema::compile(&definition.input_schema).expect("import schema compiles");
        let mut params = valid_import_params();
        params["import_report"] = json!({
            "schema_version": KG_IMPORT_REPORT_SCHEMA,
            "source_candidates_schema_version": KG_IMPORT_CANDIDATES_SCHEMA,
            "report_hash": "2222222222222222222222222222222222222222222222222222222222222222",
        });

        assert!(
            validator.validate(&params).is_err(),
            "digest-only import reports must be rejected before dispatch"
        );
    }

    #[test]
    fn export_schema_rejects_non_hash_memory_ids() {
        let definition = export_definition();
        let validator =
            JSONSchema::compile(&definition.input_schema).expect("export schema compiles");
        let mut params = valid_export_params();
        params["included_memory_ids"] = json!(["memory-001"]);

        assert!(
            validator.validate(&params).is_err(),
            "included_memory_ids must be 64-character lowercase hex hashes"
        );
    }

    // ---------------------------------------------------------------------
    // Schema-drift guard (GAP-012 P1-C, item 4): the MCP tool input schemas
    // must stay bound to the versioned exo-api DAG DB request DTOs. For each
    // tool we (a) parse the shared exo-dag-db request fixture into the DTO so a
    // DTO field rename/removal breaks here, and (b) validate the SAME fixture
    // against the compiled tool schema so a schema rename/removal breaks here.
    // Together a fixture that the DTO accepts but the schema rejects (or vice
    // versa) fails the test rather than drifting silently.
    // ---------------------------------------------------------------------
    #[test]
    fn schemas_stay_bound_to_exo_api_dtos() {
        use exo_api::dagdb::{DagDbContextPacketRequest, DagDbWritebackRequest};

        fn assert_bound<T: serde::de::DeserializeOwned>(
            definition: ToolDefinition,
            fixture_name: &str,
        ) {
            let fixture = request_fixture(fixture_name);

            // (a) The DTO accepts the fixture (binds the schema to the DTO via
            // the shared fixture as the single source of truth).
            let _dto: T = serde_json::from_value(fixture.clone()).unwrap_or_else(|err| {
                panic!("fixture {fixture_name} must deserialize into its exo-api DTO: {err}")
            });

            // (b) The compiled tool schema accepts the same fixture.
            let validator = JSONSchema::compile(&definition.input_schema)
                .unwrap_or_else(|err| panic!("{} schema compiles: {err}", definition.name));
            if let Err(errors) = validator.validate(&fixture) {
                let msgs: Vec<String> = errors.map(|err| err.to_string()).collect();
                panic!(
                    "{} input schema must accept the exo-api {fixture_name} request fixture: {}",
                    definition.name,
                    msgs.join("; ")
                );
            }
        }

        assert_bound::<DagDbContextPacketRequest>(
            get_context_packet_definition(),
            "context_packet",
        );
        assert_bound::<DagDbWritebackRequest>(submit_writeback_definition(), "writeback");
        // The import/export fixtures live in the SDK helpers, not the shared
        // request fixture set, so build representative DTOs and round-trip them
        // through both the DTO and the schema here.
        assert_import_export_bound();
    }

    fn assert_import_export_bound() {
        use exo_api::dagdb::{DagDbExportRequest, DagDbImportRequest};

        let import_params = valid_import_params();
        let _import: DagDbImportRequest = serde_json::from_value(import_params.clone())
            .expect("import params deserialize into DagDbImportRequest");
        let import_validator =
            JSONSchema::compile(&import_definition().input_schema).expect("import schema compiles");
        assert!(
            import_validator.validate(&import_params).is_ok(),
            "import schema must accept a valid DagDbImportRequest payload"
        );

        let export_params = valid_export_params();
        let _export: DagDbExportRequest = serde_json::from_value(export_params.clone())
            .expect("export params deserialize into DagDbExportRequest");
        let export_validator =
            JSONSchema::compile(&export_definition().input_schema).expect("export schema compiles");
        assert!(
            export_validator.validate(&export_params).is_ok(),
            "export schema must accept a valid DagDbExportRequest payload"
        );
    }
}
