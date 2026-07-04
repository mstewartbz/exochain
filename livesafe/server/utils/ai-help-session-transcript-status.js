"use strict";

const SOURCE_BASIS = [
  "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
  "docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md",
  "docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md",
  "src/ai_help_session_transcript.rs",
  "docs/TEST_PLAN.md"
];

const QUERY_OPERATIONS = [
  "query-ai-help-session-transcript",
  "query-ai-help-active-session-index"
];

const QUERY_SHAPE = {
  required_parameters: ["session_id"],
  transcript_result_fields: [
    "session_id",
    "created_at",
    "updated_at",
    "expires_at",
    "outcome",
    "question_summary",
    "route",
    "surface_id",
    "cited_topic_ids",
    "generated_feedback_count",
    "messages"
  ],
  message_fields: ["role", "text", "timestamp"],
  active_session_index_fields: [
    "session_id",
    "created_at",
    "updated_at",
    "expires_at",
    "outcome",
    "question_summary",
    "route",
    "surface_id",
    "cited_topic_ids",
    "generated_feedback_count"
  ],
  retention_window: "rolling-seven-days"
};

const BLOCKED_OPERATIONS = [
  "read-ai-help-session-transcript",
  "read-ai-help-usage-summary",
  "ask-ai-help",
  "create-feedback",
  "auto-create-mandated-report",
  "dispatch-agent"
];

function createAiHelpSessionTranscriptStatusPayload(options = {}) {
  return {
    status: "inactive",
    api_surface: "api-response",
    read_only: true,
    backend_selected: false,
    transcript_query_route_enabled: false,
    transcript_access_enabled: false,
    session_ttl_hours: 168,
    active_session_index_limit: 50,
    query_operations: QUERY_OPERATIONS,
    query_shape: QUERY_SHAPE,
    allowed_operations: ["read-status"],
    blocked_operations: BLOCKED_OPERATIONS,
    public_claims_allowed: false,
    source_basis: SOURCE_BASIS,
    generated_at: options.generatedAt ?? new Date().toISOString()
  };
}

function sendAiHelpSessionTranscriptStatusResponse(_req, res, options) {
  return res.status(200).json(createAiHelpSessionTranscriptStatusPayload(options));
}

module.exports = {
  createAiHelpSessionTranscriptStatusPayload,
  sendAiHelpSessionTranscriptStatusResponse
};
