import { describe, expect, it } from "vitest";

import {
  aiHelpSessionTranscriptQueryOperations,
  buildAiHelpSessionTranscriptQueryContract,
  isAiHelpSessionTranscriptQueryOperation,
  type AiHelpSessionTranscriptQueryOperation
} from "../src/ai-help-session-transcript-query.js";

describe("AI help session-transcript typed-query contract", () => {
  it("builds the bounded transcript lookup contract", () => {
    expect(
      buildAiHelpSessionTranscriptQueryContract({
        operation: "query-ai-help-session-transcript",
        sessionId: "session_123"
      })
    ).toEqual({
      operation: "query-ai-help-session-transcript",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "AI-help session-transcript query routes remain disabled until a backend is selected and tested."
      ],
      windowDays: 7,
      requiredParameters: ["sessionId"],
      resultFields: [
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
      messageFields: ["role", "text", "timestamp"],
      sessionId: "session_123"
    });
  });

  it("builds the bounded active-session index contract", () => {
    expect(
      buildAiHelpSessionTranscriptQueryContract({
        operation: "query-ai-help-active-session-index"
      })
    ).toEqual({
      operation: "query-ai-help-active-session-index",
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "AI-help session-transcript query routes remain disabled until a backend is selected and tested."
      ],
      windowDays: 7,
      requiredParameters: [],
      resultFields: [
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
      activeSessionIndexLimit: 50
    });
  });

  it("fails closed for unsupported operations and invalid parameter shapes", () => {
    expect(() =>
      buildAiHelpSessionTranscriptQueryContract({
        operation: "query-ai-help-usage-summary"
      })
    ).toThrow(
      "Unsupported AI-help session-transcript query operation: query-ai-help-usage-summary."
    );

    expect(() =>
      buildAiHelpSessionTranscriptQueryContract({
        operation: "query-ai-help-session-transcript"
      })
    ).toThrow(
      "query-ai-help-session-transcript requires a sessionId that matches /^[A-Za-z0-9_-]+$/."
    );

    expect(() =>
      buildAiHelpSessionTranscriptQueryContract({
        operation: "query-ai-help-session-transcript",
        sessionId: "session:unsafe"
      })
    ).toThrow(
      "query-ai-help-session-transcript requires a sessionId that matches /^[A-Za-z0-9_-]+$/."
    );

    expect(() =>
      buildAiHelpSessionTranscriptQueryContract({
        operation: "query-ai-help-active-session-index",
        sessionId: "session_123"
      })
    ).toThrow(
      "query-ai-help-active-session-index does not accept sessionId or windowDays overrides."
    );
  });

  it("exposes the bounded transcript query vocabulary", () => {
    const supported: AiHelpSessionTranscriptQueryOperation[] = [
      "query-ai-help-session-transcript",
      "query-ai-help-active-session-index"
    ];

    expect(aiHelpSessionTranscriptQueryOperations).toEqual(supported);
    expect(supported.filter(isAiHelpSessionTranscriptQueryOperation)).toEqual(
      supported
    );
    expect(isAiHelpSessionTranscriptQueryOperation("query-ai-help-usage-summary")).toBe(
      false
    );
  });
});
