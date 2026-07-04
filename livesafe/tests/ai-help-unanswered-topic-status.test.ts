import { describe, expect, it, vi } from "vitest";

const {
  createAiHelpUnansweredTopicStatusPayload,
  sendAiHelpUnansweredTopicStatusResponse
} = require("../server/utils/ai-help-unanswered-topic-status.js");

describe("AI help unanswered-topic status API contract", () => {
  it("builds an explicitly inactive read-only payload for unresolved-topic query inventory", () => {
    const payload = createAiHelpUnansweredTopicStatusPayload({
      generatedAt: "2026-05-26T16:55:00.000Z"
    });

    expect(payload).toMatchObject({
      status: "inactive",
      api_surface: "api-response",
      read_only: true,
      backend_selected: false,
      unanswered_topic_query_route_enabled: false,
      transcript_access_enabled: false,
      window_days: 7,
      allowed_operations: ["read-status"],
      generated_at: "2026-05-26T16:55:00.000Z"
    });

    expect(payload.query_operations).toEqual([
      "query-ai-help-unanswered-topics"
    ]);
    expect(payload.query_shape).toEqual({
      required_parameters: [],
      result_fields: [
        "topic_id",
        "unanswered_count",
        "confusion_count",
        "total_count"
      ],
      ordering: ["total_count:desc", "unanswered_count:desc", "topic_id:asc"],
      retention_window: "rolling-seven-days"
    });
    expect(payload.blocked_operations).toEqual([
      "read-ai-help-unanswered-topics",
      "read-ai-help-session-transcript",
      "read-ai-help-usage-summary",
      "ask-ai-help",
      "create-feedback",
      "auto-create-mandated-report",
      "dispatch-agent"
    ]);
    expect(payload.public_claims_allowed).toBe(false);
    expect(payload.source_basis).toEqual([
      "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
      "docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md",
      "docs/context/LIVESAFE_AI_HELP_PERSISTENCE_NAMESPACE.md",
      "src/ai_help_unanswered_topic.rs",
      "docs/TEST_PLAN.md"
    ]);
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendAiHelpUnansweredTopicStatusResponse(req, res, {
      generatedAt: "2026-05-26T16:56:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "inactive",
        read_only: true,
        unanswered_topic_query_route_enabled: false,
        transcript_access_enabled: false
      })
    );
  });
});
