"use strict";

const SOURCE_BASIS = [
  "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
  "docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md",
  "docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md",
  "src/ai_help_usage_summary.rs",
  "docs/TEST_PLAN.md"
];

const QUERY_OPERATIONS = ["query-ai-help-usage-summary"];

const QUERY_SHAPE = {
  required_parameters: [],
  result_fields: [
    "window_started_at",
    "window_ended_at",
    "total_sessions",
    "generated_feedback_count",
    "outcome_counts",
    "topic_counts",
    "top_questions",
    "unresolved_topics"
  ],
  retention_window: "rolling-seven-days"
};

const BLOCKED_OPERATIONS = [
  "read-ai-help-usage-summary",
  "read-ai-help-session-transcript",
  "ask-ai-help",
  "create-feedback",
  "auto-create-mandated-report",
  "dispatch-agent"
];

function createAiHelpUsageSummaryStatusPayload(options = {}) {
  return {
    status: "inactive",
    api_surface: "api-response",
    read_only: true,
    backend_selected: false,
    usage_summary_query_route_enabled: false,
    transcript_access_enabled: false,
    window_days: 7,
    query_operations: QUERY_OPERATIONS,
    query_shape: QUERY_SHAPE,
    allowed_operations: ["read-status"],
    blocked_operations: BLOCKED_OPERATIONS,
    public_claims_allowed: false,
    source_basis: SOURCE_BASIS,
    generated_at: options.generatedAt ?? new Date().toISOString()
  };
}

function sendAiHelpUsageSummaryStatusResponse(_req, res, options) {
  return res.status(200).json(createAiHelpUsageSummaryStatusPayload(options));
}

module.exports = {
  createAiHelpUsageSummaryStatusPayload,
  sendAiHelpUsageSummaryStatusResponse
};
