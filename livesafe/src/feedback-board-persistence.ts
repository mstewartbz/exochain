const SAFE_TOKEN_PATTERN = /^[A-Za-z0-9:_-]+$/;

const feedbackStatusOrder = [
  "new",
  "backlog",
  "planning",
  "development",
  "testing",
  "validation",
  "deployed",
  "held"
] as const;

const feedbackTargetTypes = [
  "onboarding-step",
  "pace-contact",
  "ice-card",
  "qr-activation",
  "responder-view",
  "emergency-profile",
  "medical-jacket",
  "genotypical-import",
  "consent-control",
  "vault-record",
  "ambient-signal",
  "marketplace-template",
  "entitlement-plan",
  "frontline-eligibility",
  "trust-state",
  "ui-component",
  "general"
] as const;

export const feedbackBoardPersistenceSurfaces = [
  "feedback-item",
  "feedback-board",
  "feedback-by-target",
  "feedback-by-work-batch",
  "feedback-index-all",
  "feedback-activities",
  "feedback-votes",
  "feedback-stats-by-category",
  "feedback-stats-by-target-type",
  "feedback-stats-by-status"
] as const;

export type FeedbackBoardPersistenceSurface =
  (typeof feedbackBoardPersistenceSurfaces)[number];
export type FeedbackBoardStatus = (typeof feedbackStatusOrder)[number];
export type FeedbackBoardTargetType = (typeof feedbackTargetTypes)[number];

export interface FeedbackBoardPersistenceInput {
  feedbackId?: string;
  status?: string;
  targetType?: string;
  targetId?: string;
  workBatchTag?: string;
}

export interface FeedbackBoardPersistenceNamespaceEntry {
  surface: FeedbackBoardPersistenceSurface;
  key: string;
}

export function isSupportedFeedbackBoardPersistenceSurface(
  value: string
): value is FeedbackBoardPersistenceSurface {
  return feedbackBoardPersistenceSurfaces.includes(
    value as FeedbackBoardPersistenceSurface
  );
}

export function buildFeedbackBoardPersistenceNamespace(
  surface: FeedbackBoardPersistenceSurface,
  input?: FeedbackBoardPersistenceInput
): FeedbackBoardPersistenceNamespaceEntry {
  switch (surface) {
    case "feedback-item":
      return entry(surface, [
        "livesafe",
        "feedback",
        "item",
        requireSafeToken(surface, "feedbackId", input?.feedbackId)
      ]);
    case "feedback-board":
      return entry(surface, [
        "livesafe",
        "feedback",
        "board",
        normalizeStatus(surface, input?.status)
      ]);
    case "feedback-by-target":
      return entry(surface, [
        "livesafe",
        "feedback",
        "by_target",
        normalizeTargetType(surface, input?.targetType),
        requireSafeToken(surface, "targetId", input?.targetId)
      ]);
    case "feedback-by-work-batch":
      return entry(surface, [
        "livesafe",
        "feedback",
        "by_work_batch",
        requireSafeToken(surface, "workBatchTag", input?.workBatchTag)
      ]);
    case "feedback-index-all":
      rejectUnexpectedParameters(surface, input);
      return entry(surface, ["livesafe", "feedback", "index", "all"]);
    case "feedback-activities":
      return entry(surface, [
        "livesafe",
        "feedback",
        "activities",
        requireSafeToken(surface, "feedbackId", input?.feedbackId)
      ]);
    case "feedback-votes":
      return entry(surface, [
        "livesafe",
        "feedback",
        "votes",
        requireSafeToken(surface, "feedbackId", input?.feedbackId)
      ]);
    case "feedback-stats-by-category":
      rejectUnexpectedParameters(surface, input);
      return entry(surface, ["livesafe", "feedback", "stats", "by_category"]);
    case "feedback-stats-by-target-type":
      rejectUnexpectedParameters(surface, input);
      return entry(surface, [
        "livesafe",
        "feedback",
        "stats",
        "by_target_type"
      ]);
    case "feedback-stats-by-status":
      rejectUnexpectedParameters(surface, input);
      return entry(surface, ["livesafe", "feedback", "stats", "by_status"]);
  }
}

function entry(
  surface: FeedbackBoardPersistenceSurface,
  segments: string[]
): FeedbackBoardPersistenceNamespaceEntry {
  return {
    surface,
    key: segments.join(":")
  };
}

function normalizeStatus(
  surface: "feedback-board",
  value: string | undefined
): FeedbackBoardStatus {
  if (value === undefined) {
    throw new Error(`${surface} requires status.`);
  }

  if (!feedbackStatusOrder.includes(value as FeedbackBoardStatus)) {
    throw new Error(
      `${surface} status must be one of ${feedbackStatusOrder.join(", ")}.`
    );
  }

  return value as FeedbackBoardStatus;
}

function normalizeTargetType(
  surface: "feedback-by-target",
  value: string | undefined
): FeedbackBoardTargetType {
  if (value === undefined) {
    throw new Error(`${surface} requires targetType.`);
  }

  if (!feedbackTargetTypes.includes(value as FeedbackBoardTargetType)) {
    throw new Error(`${surface} targetType is unsupported: ${value}.`);
  }

  return value as FeedbackBoardTargetType;
}

function requireSafeToken(
  surface:
    | "feedback-item"
    | "feedback-by-target"
    | "feedback-by-work-batch"
    | "feedback-activities"
    | "feedback-votes",
  field: "feedbackId" | "targetId" | "workBatchTag",
  value: string | undefined
): string {
  if (value === undefined) {
    throw new Error(`${surface} requires ${field}.`);
  }

  if (!SAFE_TOKEN_PATTERN.test(value)) {
    throw new Error(`${surface} ${field} must match /^[A-Za-z0-9:_-]+$/.`);
  }

  return value;
}

function rejectUnexpectedParameters(
  surface:
    | "feedback-index-all"
    | "feedback-stats-by-category"
    | "feedback-stats-by-target-type"
    | "feedback-stats-by-status",
  input: FeedbackBoardPersistenceInput | undefined
): void {
  if (input === undefined) {
    return;
  }

  const definedParameters = Object.values(input).filter(
    (value) => value !== undefined
  );
  if (definedParameters.length > 0) {
    throw new Error(`${surface} does not accept parameters.`);
  }
}
