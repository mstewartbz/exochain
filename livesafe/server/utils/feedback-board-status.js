"use strict";

const SOURCE_BASIS = [
  "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
  "docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md",
  "src/feedback_board_read_model.rs",
  "docs/TEST_PLAN.md"
];

const QUERY_OPERATIONS = [
  "query-feedback-board",
  "query-feedback-by-target",
  "query-feedback-by-work-batch",
  "query-feedback-item",
  "query-feedback-activity-log",
  "query-feedback-counts-by-target",
  "query-feedback-stats"
];

const BLOCKED_OPERATIONS = [
  "read-feedback-board",
  "read-feedback-by-target",
  "read-feedback-by-work-batch",
  "read-feedback-item",
  "read-feedback-activity-log",
  "read-feedback-counts-by-target",
  "read-feedback-stats",
  "create-feedback",
  "update-feedback-status",
  "update-feedback-priority",
  "assign-feedback-work-batch",
  "reject-feedback-validation",
  "accept-feedback-deployment",
  "hold-feedback",
  "unhold-feedback",
  "comment-feedback",
  "upvote-feedback",
  "delete-feedback"
];

function createFeedbackBoardStatusPayload(options = {}) {
  return {
    status: "inactive",
    api_surface: "api-response",
    read_only: true,
    backend_selected: false,
    board_query_routes_enabled: false,
    feedback_write_routes_enabled: false,
    query_operations: QUERY_OPERATIONS,
    allowed_operations: ["read-status"],
    blocked_operations: BLOCKED_OPERATIONS,
    public_claims_allowed: false,
    source_basis: SOURCE_BASIS,
    generated_at: options.generatedAt ?? new Date().toISOString()
  };
}

function sendFeedbackBoardStatusResponse(_req, res, options) {
  return res.status(200).json(createFeedbackBoardStatusPayload(options));
}

module.exports = {
  createFeedbackBoardStatusPayload,
  sendFeedbackBoardStatusResponse
};
