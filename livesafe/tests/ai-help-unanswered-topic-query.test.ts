import { describe, expect, it } from "vitest";

import {
  aiHelpUnansweredTopicQueryOperations,
  buildAiHelpUnansweredTopicQueryContract,
  isAiHelpUnansweredTopicQueryOperation,
  type AiHelpUnansweredTopicQueryOperation
} from "../src/ai-help-unanswered-topic-query.js";

describe("AI help unanswered-topic typed-query contract", () => {
  it("builds the bounded unanswered-topic query contract", () => {
    expect(
      buildAiHelpUnansweredTopicQueryContract({
        operation: "query-ai-help-unanswered-topics"
      })
    ).toEqual({
      operation: "query-ai-help-unanswered-topics",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "AI-help unanswered-topic query routes remain disabled until a backend is selected and tested."
      ],
      windowDays: 7,
      requiredParameters: [],
      resultFields: [
        "topic_id",
        "unanswered_count",
        "confusion_count",
        "total_count"
      ],
      ordering: [
        "total_count:desc",
        "unanswered_count:desc",
        "confusion_count:desc",
        "topic_id:asc"
      ]
    });
  });

  it("fails closed for unsupported operations and extra parameters", () => {
    expect(() =>
      buildAiHelpUnansweredTopicQueryContract({
        operation: "query-ai-help-usage-summary"
      })
    ).toThrow(
      "Unsupported AI-help unanswered-topic query operation: query-ai-help-usage-summary."
    );

    expect(() =>
      buildAiHelpUnansweredTopicQueryContract({
        operation: "query-ai-help-unanswered-topics",
        topicId: "topic_123"
      })
    ).toThrow(
      "query-ai-help-unanswered-topics does not accept topicId, sessionId, or windowDays overrides."
    );
  });

  it("exposes the bounded unanswered-topic query vocabulary", () => {
    const supported: AiHelpUnansweredTopicQueryOperation[] = [
      "query-ai-help-unanswered-topics"
    ];

    expect(aiHelpUnansweredTopicQueryOperations).toEqual(supported);
    expect(supported.filter(isAiHelpUnansweredTopicQueryOperation)).toEqual(
      supported
    );
    expect(isAiHelpUnansweredTopicQueryOperation("query-ai-help-usage-summary")).toBe(
      false
    );
  });
});
