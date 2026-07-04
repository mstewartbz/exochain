import { describe, expect, it, vi } from "vitest";

const {
  createAiHelpSessionTranscriptStatusPayload,
  sendAiHelpSessionTranscriptStatusResponse
} = require("../server/utils/ai-help-session-transcript-status.js");

describe("AI help session transcript status API contract", () => {
  it("builds an explicitly inactive read-only payload for transcript query inventory", () => {
    const payload = createAiHelpSessionTranscriptStatusPayload({
      generatedAt: "2026-05-26T16:40:00.000Z"
    });

    expect(payload).toMatchObject({
      status: "inactive",
      api_surface: "api-response",
      read_only: true,
      backend_selected: false,
      transcript_query_route_enabled: false,
      transcript_access_enabled: false,
      session_ttl_hours: 168,
      active_session_index_limit: 50,
      allowed_operations: ["read-status"],
      generated_at: "2026-05-26T16:40:00.000Z"
    });

    expect(payload.query_operations).toEqual([
      "query-ai-help-session-transcript",
      "query-ai-help-active-session-index"
    ]);
    expect(payload.query_shape).toEqual({
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
    });
    expect(payload.blocked_operations).toEqual([
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
      "src/ai_help_session_transcript.rs",
      "docs/TEST_PLAN.md"
    ]);
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendAiHelpSessionTranscriptStatusResponse(req, res, {
      generatedAt: "2026-05-26T16:41:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "inactive",
        read_only: true,
        transcript_query_route_enabled: false,
        transcript_access_enabled: false
      })
    );
  });
});
