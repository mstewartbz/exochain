import { describe, expect, it, vi } from "vitest";

import {
  createAiHelpStatusPayload,
  sendAiHelpStatusResponse
} from "../server/utils/ai-help-status.js";

describe("AI help status API contract", () => {
  it("builds an explicitly inactive feature-gate payload by default", () => {
    const payload = createAiHelpStatusPayload({
      generatedAt: "2026-05-26T10:40:00.000Z"
    });

    expect(payload).toMatchObject({
      status: "inactive",
      api_surface: "api-response",
      read_only: true,
      write_routes_enabled: false,
      help_ai_enabled: false,
      feedback_writes_enabled: false,
      help_ai_mandated_reporter_enabled: false,
      feedback_agent_dispatch_enabled: false,
      feedback_screenshots_enabled: false,
      feedback_code_hints_enabled: false,
      feedback_agent_trigger_statuses: ["DEVELOPMENT"],
      help_ai_session_ttl_hours: 168,
      help_ai_report_interval_minutes: 15,
      help_ai_unanswered_threshold: 3,
      generated_at: "2026-05-26T10:40:00.000Z"
    });

    expect(payload.blocked_operations).toEqual([
      "ask-ai-help",
      "create-feedback",
      "auto-create-mandated-report",
      "dispatch-agent"
    ]);
    expect(payload.allowed_operations).toEqual(["read-status"]);
    expect(payload.source_basis).toEqual([
      "docs/LIVESAFE_AI_HELP_FEEDBACK_AGENT_REQUIREMENTS.md",
      "docs/context/LIVESAFE_AI_HELP_FEEDBACK_AND_AGENT_SYSTEM.md",
      "docs/TEST_PLAN.md"
    ]);
  });

  it("parses explicit environment-style flags and thresholds without enabling writes implicitly", () => {
    const payload = createAiHelpStatusPayload({
      helpAiEnabled: "true",
      feedbackWritesEnabled: "false",
      helpAiMandatedReporterEnabled: "true",
      feedbackAgentDispatchEnabled: "true",
      feedbackScreenshotsEnabled: "true",
      feedbackCodeHintsEnabled: "true",
      feedbackAgentTriggerStatuses: "DEVELOPMENT,TESTING",
      helpAiSessionTtlHours: "72",
      helpAiReportIntervalMinutes: "30",
      helpAiUnansweredThreshold: "5",
      generatedAt: "2026-05-26T10:41:00.000Z"
    });

    expect(payload.help_ai_enabled).toBe(true);
    expect(payload.feedback_writes_enabled).toBe(false);
    expect(payload.help_ai_mandated_reporter_enabled).toBe(true);
    expect(payload.feedback_agent_dispatch_enabled).toBe(true);
    expect(payload.feedback_screenshots_enabled).toBe(true);
    expect(payload.feedback_code_hints_enabled).toBe(true);
    expect(payload.feedback_agent_trigger_statuses).toEqual([
      "DEVELOPMENT",
      "TESTING"
    ]);
    expect(payload.help_ai_session_ttl_hours).toBe(72);
    expect(payload.help_ai_report_interval_minutes).toBe(30);
    expect(payload.help_ai_unanswered_threshold).toBe(5);
    expect(payload.read_only).toBe(true);
    expect(payload.write_routes_enabled).toBe(false);
  });

  it("returns the payload through a read-only handler", () => {
    const req = {};
    const json = vi.fn();
    const status = vi.fn(() => ({ json }));
    const res = { status };

    sendAiHelpStatusResponse(req, res, {
      generatedAt: "2026-05-26T10:42:00.000Z"
    });

    expect(status).toHaveBeenCalledWith(200);
    expect(json).toHaveBeenCalledWith(
      expect.objectContaining({
        status: "inactive",
        read_only: true,
        write_routes_enabled: false
      })
    );
  });
});
