import { describe, expect, it, vi } from "vitest";

import {
  createFeedbackBoardStatusPayload,
  sendFeedbackBoardStatusResponse
} from "../server/utils/feedback-board-status.js";

describe("feedback board status API contract", () => {
  it("builds an explicitly inactive read-only payload for board query operations", () => {
    const payload = createFeedbackBoardStatusPayload({
      generatedAt: "2026-05-26T15:30:00.000Z"
    });

    expect(payload).toMatchObject({
      status: "inactive",
      api_surface: "api-response",
      read_only: true,
      board_query_routes_enabled: false,
      feedback_write_routes_enabled: false,
      allowed_operations: ["read-status"],
      generated_at: "2026-05-26T15:30:00.000Z"
    });

    expect(payload.query_operations).toEqual([
      "query-feedback-board",
      "query-feedback-by-target",
      "query-feedback-by-work-batch",
      "query-feedback-item",
      "query-feedback-activity-log",
      "query-feedback-counts-by-target",
      "query-feedback-stats"
    ]);
    expect(payload.blocked_operations).toEqual([
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
    ]);
    expect(payload.backend_selected).toBe(false);
    expect(payload.public_claims_allowed).toBe(false);
    expect(payload.source_basis).toEqual([
      "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
      "docs/context/LIVESAFE_AI_HELP_TYPED_ROUTE_OPERATIONS.md",
      "src/feedback_board_read_model.rs",
      "docs/TEST_PLAN.md"
    ]);
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendFeedbackBoardStatusResponse(req, res, {
      generatedAt: "2026-05-26T15:31:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "inactive",
        read_only: true,
        board_query_routes_enabled: false,
        feedback_write_routes_enabled: false
      })
    );
  });
});
