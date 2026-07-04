const SAFE_TOKEN_PATTERN = /^[A-Za-z0-9:_-]+$/;

export const feedbackBoardReadOperations = [
  "query-feedback-board",
  "query-feedback-by-target",
  "query-feedback-by-work-batch",
  "query-feedback-item",
  "query-feedback-activity-log",
  "query-feedback-counts-by-target",
  "query-feedback-stats"
] as const;

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

const feedbackCategories = [
  "bug",
  "feature-request",
  "documentation-gap",
  "data-quality",
  "ui-ux",
  "performance",
  "entitlement-billing",
  "privacy-safety",
  "other"
] as const;

export type FeedbackBoardReadOperation =
  (typeof feedbackBoardReadOperations)[number];
export type FeedbackStatusFilter = (typeof feedbackStatusOrder)[number];
export type FeedbackTargetTypeFilter = (typeof feedbackTargetTypes)[number];
export type FeedbackCategoryFilter = (typeof feedbackCategories)[number];

export interface FeedbackBoardReadContractInput {
  operation: string;
  statuses?: string[];
  targetType?: string;
  category?: string;
  workBatchTag?: string;
  targetId?: string;
  feedbackId?: string;
}

export interface FeedbackBoardReadFilters {
  statuses: FeedbackStatusFilter[];
  targetType?: FeedbackTargetTypeFilter;
  category?: FeedbackCategoryFilter;
  workBatchTag?: string;
  targetId?: string;
  feedbackId?: string;
}

export interface FeedbackBoardReadContract {
  operation: FeedbackBoardReadOperation;
  readOnly: true;
  routeEnabled: false;
  executionAllowed: false;
  blockers: readonly [
    "Feedback-board query routes remain disabled until a backend is selected and tested."
  ];
  filters: FeedbackBoardReadFilters;
}

export function isFeedbackBoardReadOperation(
  value: string
): value is FeedbackBoardReadOperation {
  return feedbackBoardReadOperations.includes(
    value as FeedbackBoardReadOperation
  );
}

export function buildFeedbackBoardReadContract(
  input: FeedbackBoardReadContractInput
): FeedbackBoardReadContract {
  if (!isFeedbackBoardReadOperation(input.operation)) {
    throw new Error(
      `Unsupported feedback-board read operation: ${input.operation}.`
    );
  }

  const filters: FeedbackBoardReadFilters = {
    statuses: normalizeStatuses(input.statuses)
  };

  if (input.targetType !== undefined) {
    filters.targetType = normalizeTargetType(input.targetType);
  }

  if (input.category !== undefined) {
    filters.category = normalizeCategory(input.category);
  }

  if (input.workBatchTag !== undefined) {
    filters.workBatchTag = normalizeSafeToken(
      input.operation,
      "workBatchTag",
      input.workBatchTag
    );
  }

  switch (input.operation) {
    case "query-feedback-by-target": {
      if (filters.targetType === undefined) {
        throw new Error("query-feedback-by-target requires targetType.");
      }

      if (input.targetId === undefined) {
        throw new Error("query-feedback-by-target requires targetId.");
      }

      filters.targetId = normalizeSafeToken(
        input.operation,
        "targetId",
        input.targetId
      );
      break;
    }
    case "query-feedback-by-work-batch": {
      if (filters.workBatchTag === undefined) {
        throw new Error("query-feedback-by-work-batch requires workBatchTag.");
      }
      break;
    }
    case "query-feedback-item":
    case "query-feedback-activity-log": {
      if (input.feedbackId === undefined) {
        throw new Error(`${input.operation} requires feedbackId.`);
      }

      filters.feedbackId = normalizeSafeToken(
        input.operation,
        "feedbackId",
        input.feedbackId
      );
      break;
    }
  }

  return {
    operation: input.operation,
    readOnly: true,
    routeEnabled: false,
    executionAllowed: false,
    blockers: [
      "Feedback-board query routes remain disabled until a backend is selected and tested."
    ],
    filters
  };
}

function normalizeStatuses(values: string[] | undefined): FeedbackStatusFilter[] {
  if (values === undefined) {
    return [];
  }

  const provided = new Set<FeedbackStatusFilter>();
  for (const value of values) {
    if (!feedbackStatusOrder.includes(value as FeedbackStatusFilter)) {
      throw new Error(`Unsupported feedback status filter: ${value}.`);
    }

    provided.add(value as FeedbackStatusFilter);
  }

  return feedbackStatusOrder.filter((status) => provided.has(status));
}

function normalizeTargetType(value: string): FeedbackTargetTypeFilter {
  if (!feedbackTargetTypes.includes(value as FeedbackTargetTypeFilter)) {
    throw new Error(`Unsupported feedback targetType filter: ${value}.`);
  }

  return value as FeedbackTargetTypeFilter;
}

function normalizeCategory(value: string): FeedbackCategoryFilter {
  if (!feedbackCategories.includes(value as FeedbackCategoryFilter)) {
    throw new Error(`Unsupported feedback category filter: ${value}.`);
  }

  return value as FeedbackCategoryFilter;
}

function normalizeSafeToken(
  operation: FeedbackBoardReadOperation,
  field: "workBatchTag" | "targetId" | "feedbackId",
  value: string
): string {
  if (!SAFE_TOKEN_PATTERN.test(value)) {
    throw new Error(
      `${operation} ${field} must match /^[A-Za-z0-9:_-]+$/.`
    );
  }

  return value;
}
