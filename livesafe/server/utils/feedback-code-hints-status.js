"use strict";

const SOURCE_BASIS = [
  "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
  "docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md",
  "src/feedback-code-hints.ts",
  "docs/TEST_PLAN.md"
];

const BLOCKED_OPERATIONS = [
  "read-feedback-code-hints",
  "generate-feedback-code-hints",
  "persist-feedback-code-hints",
  "dispatch-feedback-code-hints"
];

const SUPPORTED_COMPONENTS = [
  "onboarding-wizard",
  "pace-invite-flow",
  "ice-card-generator",
  "qr-activation",
  "responder-view",
  "emergency-profile-editor",
  "medical-jacket",
  "consent-controls",
  "marketplace-template-config",
  "entitlement-plan-selector",
  "frontline-eligibility",
  "trust-state-banner"
];

const CODE_HINT_FIELDS = [
  "service",
  "filePaths",
  "specRef",
  "storageKeys",
  "apiOperation"
];

function createFeedbackCodeHintsStatusPayload(options = {}) {
  return {
    status: "inactive",
    api_surface: "api-response",
    read_only: true,
    code_hints_route_enabled: false,
    code_hints_registry_enabled: false,
    supported_components: SUPPORTED_COMPONENTS,
    code_hints_fields: CODE_HINT_FIELDS,
    allowed_operations: ["read-status"],
    blocked_operations: BLOCKED_OPERATIONS,
    public_claims_allowed: false,
    source_basis: SOURCE_BASIS,
    generated_at: options.generatedAt ?? new Date().toISOString()
  };
}

function sendFeedbackCodeHintsStatusResponse(_req, res, options) {
  return res.status(200).json(createFeedbackCodeHintsStatusPayload(options));
}

module.exports = {
  createFeedbackCodeHintsStatusPayload,
  sendFeedbackCodeHintsStatusResponse
};
