const DAYS_TO_MS = 24 * 60 * 60 * 1000;
const DAYS_TO_SECONDS = 24 * 60 * 60;
const SAFE_ID_PATTERN = /^[A-Za-z0-9_-]+$/;

export const AI_HELP_RETENTION_DAYS = 7;
export const AI_HELP_RETENTION_MS = AI_HELP_RETENTION_DAYS * DAYS_TO_MS;
export const AI_HELP_RETENTION_SECONDS =
  AI_HELP_RETENTION_DAYS * DAYS_TO_SECONDS;

export type AiHelpPersistenceSurface =
  | "help-session"
  | "help-messages"
  | "recent-session-index"
  | "unanswered-topic";

export interface AiHelpPersistenceNamespaceEntry {
  surface: AiHelpPersistenceSurface;
  key: string;
  ttlSeconds: number;
}

const AI_HELP_SURFACE_SEGMENTS: Record<AiHelpPersistenceSurface, string[]> = {
  "help-session": ["livesafe", "help", "session"],
  "help-messages": ["livesafe", "help", "session"],
  "recent-session-index": ["livesafe", "help", "sessions", "recent"],
  "unanswered-topic": ["livesafe", "help", "topic", "unanswered"]
};

export function isSupportedAiHelpPersistenceSurface(
  value: string
): value is AiHelpPersistenceSurface {
  return value in AI_HELP_SURFACE_SEGMENTS;
}

export function buildAiHelpPersistenceNamespace(
  surface: AiHelpPersistenceSurface,
  id?: string
): AiHelpPersistenceNamespaceEntry {
  if (surface === "recent-session-index") {
    if (id !== undefined) {
      throw new Error("recent-session-index does not accept an id.");
    }

    return {
      surface,
      key: AI_HELP_SURFACE_SEGMENTS[surface].join(":"),
      ttlSeconds: AI_HELP_RETENTION_SECONDS
    };
  }

  if (id === undefined) {
    throw new Error(`${surface} requires an id.`);
  }

  if (!SAFE_ID_PATTERN.test(id)) {
    throw new Error(`${surface} id must match /^[A-Za-z0-9_-]+$/.`);
  }

  const segments = [...AI_HELP_SURFACE_SEGMENTS[surface], id];

  if (surface === "help-messages") {
    segments.push("messages");
  }

  return {
    surface,
    key: segments.join(":"),
    ttlSeconds: AI_HELP_RETENTION_SECONDS
  };
}
