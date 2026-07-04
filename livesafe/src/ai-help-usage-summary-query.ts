export const aiHelpUsageSummaryQueryOperations = [
  "query-ai-help-usage-summary"
] as const;

const RESULT_FIELDS = [
  "window_started_at",
  "window_ended_at",
  "total_sessions",
  "generated_feedback_count",
  "outcome_counts",
  "topic_counts",
  "top_questions",
  "unresolved_topics"
] as const;

export type AiHelpUsageSummaryQueryOperation =
  (typeof aiHelpUsageSummaryQueryOperations)[number];

export interface AiHelpUsageSummaryQueryContractInput {
  operation: string;
  sessionId?: string;
  topicId?: string;
  windowDays?: number;
}

export interface AiHelpUsageSummaryQueryContract {
  operation: AiHelpUsageSummaryQueryOperation;
  readOnly: true;
  routeEnabled: false;
  executionAllowed: false;
  blockers: readonly [
    "AI-help usage-summary query routes remain disabled until a backend is selected and tested."
  ];
  windowDays: 7;
  requiredParameters: readonly [];
  resultFields: readonly (typeof RESULT_FIELDS)[number][];
}

export function isAiHelpUsageSummaryQueryOperation(
  value: string
): value is AiHelpUsageSummaryQueryOperation {
  return aiHelpUsageSummaryQueryOperations.includes(
    value as AiHelpUsageSummaryQueryOperation
  );
}

export function buildAiHelpUsageSummaryQueryContract(
  input: AiHelpUsageSummaryQueryContractInput
): AiHelpUsageSummaryQueryContract {
  if (!isAiHelpUsageSummaryQueryOperation(input.operation)) {
    throw new Error(
      `Unsupported AI-help usage-summary query operation: ${input.operation}.`
    );
  }

  if (
    input.sessionId !== undefined ||
    input.topicId !== undefined ||
    input.windowDays !== undefined
  ) {
    throw new Error(
      "query-ai-help-usage-summary does not accept sessionId, topicId, or windowDays overrides."
    );
  }

  return {
    operation: input.operation,
    readOnly: true,
    routeEnabled: false,
    executionAllowed: false,
    blockers: [
      "AI-help usage-summary query routes remain disabled until a backend is selected and tested."
    ],
    windowDays: 7,
    requiredParameters: [],
    resultFields: [...RESULT_FIELDS]
  };
}
