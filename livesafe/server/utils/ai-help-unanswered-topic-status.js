"use strict";

const SOURCE_BASIS = [
  "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
  "docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md",
  "docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md",
  "src/ai_help_unanswered_topic.rs",
  "docs/TEST_PLAN.md"
];

const QUERY_OPERATIONS = ["query-ai-help-unanswered-topics"];

const QUERY_SHAPE = {
  required_parameters: [],
  result_fields: [
    "topic_id",
    "unanswered_count",
    "confusion_count",
    "total_count"
  ],
  ordering: ["total_count:desc", "unanswered_count:desc", "topic_id:asc"],
  retention_window: "rolling-seven-days"
};

const BLOCKED_OPERATIONS = [
  "read-ai-help-unanswered-topics",
  "read-ai-help-session-transcript",
  "read-ai-help-usage-summary",
  "ask-ai-help",
  "create-feedback",
  "auto-create-mandated-report",
  "dispatch-agent"
];

function createAiHelpUnansweredTopicStatusPayload(options = {}) {
  return {
    status: "inactive",
    api_surface: "api-response",
    read_only: true,
    backend_selected: false,
    unanswered_topic_query_route_enabled: false,
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

function sendAiHelpUnansweredTopicStatusResponse(_req, res, options) {
  return res
    .status(200)
    .json(createAiHelpUnansweredTopicStatusPayload(options));
}

module.exports = {
  createAiHelpUnansweredTopicStatusPayload,
  sendAiHelpUnansweredTopicStatusResponse
};
