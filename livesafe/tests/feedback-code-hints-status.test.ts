import { describe, expect, it, vi } from "vitest";

import {
  feedbackCodeHintComponents,
  feedbackCodeHintFields
} from "../src/feedback-code-hints.js";
import {
  createFeedbackCodeHintsStatusPayload,
  sendFeedbackCodeHintsStatusResponse
} from "../server/utils/feedback-code-hints-status.js";

describe("feedback code-hints status API contract", () => {
  it("builds an explicitly inactive read-only payload for the approved code-hint inventory", () => {
    const payload = createFeedbackCodeHintsStatusPayload({
      generatedAt: "2026-06-01T02:00:00.000Z"
    });

    expect(payload).toMatchObject({
      status: "inactive",
      api_surface: "api-response",
      read_only: true,
      code_hints_route_enabled: false,
      code_hints_registry_enabled: false,
      allowed_operations: ["read-status"],
      generated_at: "2026-06-01T02:00:00.000Z"
    });

    expect(payload.supported_components).toEqual(feedbackCodeHintComponents);
    expect(payload.code_hints_fields).toEqual(feedbackCodeHintFields);
    expect(payload.blocked_operations).toEqual([
      "read-feedback-code-hints",
      "generate-feedback-code-hints",
      "persist-feedback-code-hints",
      "dispatch-feedback-code-hints"
    ]);
    expect(payload.public_claims_allowed).toBe(false);
    expect(payload.source_basis).toEqual([
      "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
      "docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md",
      "src/feedback-code-hints.ts",
      "docs/TEST_PLAN.md"
    ]);
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendFeedbackCodeHintsStatusResponse(req, res, {
      generatedAt: "2026-06-01T02:01:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "inactive",
        read_only: true,
        code_hints_route_enabled: false,
        code_hints_registry_enabled: false
      })
    );
  });
});
