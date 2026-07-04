import { describe, expect, it, vi } from "vitest";

const {
  createAiHelpUsageSummaryStatusPayload,
  sendAiHelpUsageSummaryStatusResponse
} = require("../server/utils/ai-help-usage-summary-status.js");

describe("AI help usage summary status API contract", () => {
  it("builds an explicitly inactive read-only payload for the usage-summary query shape", () => {
    const payload = createAiHelpUsageSummaryStatusPayload({
      generatedAt: "2026-05-26T16:05:00.000Z"
    });

    expect(payload).toMatchObject({
      status: "inactive",
      api_surface: "api-response",
      read_only: true,
      backend_selected: false,
      usage_summary_query_route_enabled: false,
      transcript_access_enabled: false,
      window_days: 7,
      allowed_operations: ["read-status"],
      generated_at: "2026-05-26T16:05:00.000Z"
    });

    expect(payload.query_operations).toEqual(["query-ai-help-usage-summary"]);
    expect(payload.query_shape).toEqual({
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
    });
    expect(payload.blocked_operations).toEqual([
      "read-ai-help-usage-summary",
      "read-ai-help-session-transcript",
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
      "src/ai_help_usage_summary.rs",
      "docs/TEST_PLAN.md"
    ]);
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendAiHelpUsageSummaryStatusResponse(req, res, {
      generatedAt: "2026-05-26T16:06:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "inactive",
        read_only: true,
        usage_summary_query_route_enabled: false,
        transcript_access_enabled: false
      })
    );
  });
});
