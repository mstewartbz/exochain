import { describe, expect, it } from "vitest";

import {
  aiHelpUsageSummaryQueryOperations,
  buildAiHelpUsageSummaryQueryContract,
  isAiHelpUsageSummaryQueryOperation,
  type AiHelpUsageSummaryQueryOperation
} from "../src/ai-help-usage-summary-query.js";

describe("AI help usage-summary typed-query contract", () => {
  it("builds the bounded seven-day usage-summary query contract", () => {
    expect(
      buildAiHelpUsageSummaryQueryContract({
        operation: "query-ai-help-usage-summary"
      })
    ).toEqual({
      operation: "query-ai-help-usage-summary",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "AI-help usage-summary query routes remain disabled until a backend is selected and tested."
      ],
      windowDays: 7,
      requiredParameters: [],
      resultFields: [
        "window_started_at",
        "window_ended_at",
        "total_sessions",
        "generated_feedback_count",
        "outcome_counts",
        "topic_counts",
        "top_questions",
        "unresolved_topics"
      ]
    });
  });

  it("fails closed for unsupported operations or extra parameters", () => {
    expect(() =>
      buildAiHelpUsageSummaryQueryContract({
        operation: "query-ai-help-session-transcript"
      })
    ).toThrow(
      "Unsupported AI-help usage-summary query operation: query-ai-help-session-transcript."
    );

    expect(() =>
      buildAiHelpUsageSummaryQueryContract({
        operation: "query-ai-help-usage-summary",
        sessionId: "session:123"
      })
    ).toThrow(
      "query-ai-help-usage-summary does not accept sessionId, topicId, or windowDays overrides."
    );
  });

  it("exposes the bounded usage-summary query vocabulary", () => {
    const supported: AiHelpUsageSummaryQueryOperation[] = [
      "query-ai-help-usage-summary"
    ];

    expect(aiHelpUsageSummaryQueryOperations).toEqual(supported);
    expect(supported.filter(isAiHelpUsageSummaryQueryOperation)).toEqual(
      supported
    );
    expect(isAiHelpUsageSummaryQueryOperation("query-ai-help-session-transcript")).toBe(
      false
    );
  });
});
