import {
  AI_HELP_RETENTION_MS,
  AI_HELP_RETENTION_SECONDS,
  buildAiHelpPersistenceNamespace,
  isSupportedAiHelpPersistenceSurface,
  type AiHelpPersistenceSurface
} from "../src/ai-help-persistence.js";

describe("AI help persistence namespace", () => {
  it("builds the required livesafe key inventory for supported surfaces", () => {
    const sessionId = "session_2026_05_26_alpha";
    const topicId = "topic_ai_help_runtime";

    expect(buildAiHelpPersistenceNamespace("help-session", sessionId)).toEqual({
      surface: "help-session",
      key: "livesafe:help:session:session_2026_05_26_alpha",
      ttlSeconds: AI_HELP_RETENTION_SECONDS
    });

    expect(buildAiHelpPersistenceNamespace("help-messages", sessionId)).toEqual({
      surface: "help-messages",
      key: "livesafe:help:session:session_2026_05_26_alpha:messages",
      ttlSeconds: AI_HELP_RETENTION_SECONDS
    });

    expect(buildAiHelpPersistenceNamespace("recent-session-index")).toEqual({
      surface: "recent-session-index",
      key: "livesafe:help:sessions:recent",
      ttlSeconds: AI_HELP_RETENTION_SECONDS
    });

    expect(buildAiHelpPersistenceNamespace("unanswered-topic", topicId)).toEqual({
      surface: "unanswered-topic",
      key: "livesafe:help:topic:unanswered:topic_ai_help_runtime",
      ttlSeconds: AI_HELP_RETENTION_SECONDS
    });
  });

  it("keeps the seven-day retention boundary executable", () => {
    expect(AI_HELP_RETENTION_MS).toBe(7 * 24 * 60 * 60 * 1000);
    expect(AI_HELP_RETENTION_SECONDS).toBe(7 * 24 * 60 * 60);
    expect(AI_HELP_RETENTION_SECONDS * 1000).toBe(AI_HELP_RETENTION_MS);
  });

  it("fails closed for malformed ids or unsupported id requirements", () => {
    expect(() =>
      buildAiHelpPersistenceNamespace("help-session", "bad/session")
    ).toThrow("help-session id must match /^[A-Za-z0-9_-]+$/.");

    expect(() =>
      buildAiHelpPersistenceNamespace("unanswered-topic", "topic:unsafe")
    ).toThrow("unanswered-topic id must match /^[A-Za-z0-9_-]+$/.");

    expect(() =>
      buildAiHelpPersistenceNamespace("recent-session-index", "unexpected")
    ).toThrow("recent-session-index does not accept an id.");

    expect(() =>
      buildAiHelpPersistenceNamespace("help-messages")
    ).toThrow("help-messages requires an id.");
  });

  it("exposes a bounded supported-surface vocabulary", () => {
    const supported: AiHelpPersistenceSurface[] = [
      "help-session",
      "help-messages",
      "recent-session-index",
      "unanswered-topic"
    ];

    expect(supported.filter(isSupportedAiHelpPersistenceSurface)).toEqual(supported);
    expect(isSupportedAiHelpPersistenceSurface("feedback-board")).toBe(false);
  });
});
