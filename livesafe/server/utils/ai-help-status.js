"use strict";

const SOURCE_BASIS = [
  "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
  "docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md",
  "docs/TEST_PLAN.md"
];

function parseBooleanFlag(value, defaultValue = false) {
  if (value === undefined || value === null || value === "") {
    return defaultValue;
  }

  if (typeof value === "boolean") {
    return value;
  }

  const normalized = String(value).trim().toLowerCase();
  if (["true", "1", "yes", "on"].includes(normalized)) {
    return true;
  }
  if (["false", "0", "no", "off"].includes(normalized)) {
    return false;
  }

  return defaultValue;
}

function parsePositiveInteger(value, defaultValue) {
  const parsed = Number.parseInt(String(value ?? ""), 10);
  return Number.isFinite(parsed) && parsed > 0 ? parsed : defaultValue;
}

function parseTriggerStatuses(value) {
  const raw = String(value ?? "DEVELOPMENT");
  const statuses = raw
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);

  return statuses.length > 0 ? statuses : ["DEVELOPMENT"];
}

function createAiHelpStatusPayload(options = {}) {
  return {
    status: "inactive",
    api_surface: "api-response",
    read_only: true,
    write_routes_enabled: false,
    help_ai_enabled: parseBooleanFlag(options.helpAiEnabled, false),
    feedback_writes_enabled: parseBooleanFlag(options.feedbackWritesEnabled, false),
    help_ai_mandated_reporter_enabled: parseBooleanFlag(
      options.helpAiMandatedReporterEnabled,
      false
    ),
    feedback_agent_dispatch_enabled: parseBooleanFlag(
      options.feedbackAgentDispatchEnabled,
      false
    ),
    feedback_screenshots_enabled: parseBooleanFlag(
      options.feedbackScreenshotsEnabled,
      false
    ),
    feedback_code_hints_enabled: parseBooleanFlag(options.feedbackCodeHintsEnabled, false),
    feedback_agent_trigger_statuses: parseTriggerStatuses(
      options.feedbackAgentTriggerStatuses
    ),
    help_ai_session_ttl_hours: parsePositiveInteger(options.helpAiSessionTtlHours, 168),
    help_ai_report_interval_minutes: parsePositiveInteger(
      options.helpAiReportIntervalMinutes,
      15
    ),
    help_ai_unanswered_threshold: parsePositiveInteger(
      options.helpAiUnansweredThreshold,
      3
    ),
    allowed_operations: ["read-status"],
    blocked_operations: [
      "ask-ai-help",
      "create-feedback",
      "auto-create-mandated-report",
      "dispatch-agent"
    ],
    public_claims_allowed: false,
    source_basis: SOURCE_BASIS,
    generated_at: options.generatedAt ?? new Date().toISOString()
  };
}

function sendAiHelpStatusResponse(_req, res, options) {
  return res.status(200).json(createAiHelpStatusPayload(options));
}

module.exports = {
  createAiHelpStatusPayload,
  sendAiHelpStatusResponse
};
