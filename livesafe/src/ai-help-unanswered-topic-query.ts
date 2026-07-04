const RESULT_FIELDS = [
  "topic_id",
  "unanswered_count",
  "confusion_count",
  "total_count"
] as const;

const ORDERING = [
  "total_count:desc",
  "unanswered_count:desc",
  "confusion_count:desc",
  "topic_id:asc"
] as const;

export const aiHelpUnansweredTopicQueryOperations = [
  "query-ai-help-unanswered-topics"
] as const;

export type AiHelpUnansweredTopicQueryOperation =
  (typeof aiHelpUnansweredTopicQueryOperations)[number];

export interface AiHelpUnansweredTopicQueryContractInput {
  operation: string;
  topicId?: string;
  sessionId?: string;
  windowDays?: number;
}

export interface AiHelpUnansweredTopicQueryContract {
  operation: AiHelpUnansweredTopicQueryOperation;
  readOnly: true;
  routeEnabled: false;
  executionAllowed: false;
  blockers: readonly [
    "AI-help unanswered-topic query routes remain disabled until a backend is selected and tested."
  ];
  windowDays: 7;
  requiredParameters: readonly [];
  resultFields: readonly (typeof RESULT_FIELDS)[number][];
  ordering: readonly (typeof ORDERING)[number][];
}

export function isAiHelpUnansweredTopicQueryOperation(
  value: string
): value is AiHelpUnansweredTopicQueryOperation {
  return aiHelpUnansweredTopicQueryOperations.includes(
    value as AiHelpUnansweredTopicQueryOperation
  );
}

export function buildAiHelpUnansweredTopicQueryContract(
  input: AiHelpUnansweredTopicQueryContractInput
): AiHelpUnansweredTopicQueryContract {
  if (!isAiHelpUnansweredTopicQueryOperation(input.operation)) {
    throw new Error(
      `Unsupported AI-help unanswered-topic query operation: ${input.operation}.`
    );
  }

  if (
    input.topicId !== undefined ||
    input.sessionId !== undefined ||
    input.windowDays !== undefined
  ) {
    throw new Error(
      "query-ai-help-unanswered-topics does not accept topicId, sessionId, or windowDays overrides."
    );
  }

  return {
    operation: input.operation,
    readOnly: true,
    routeEnabled: false,
    executionAllowed: false,
    blockers: [
      "AI-help unanswered-topic query routes remain disabled until a backend is selected and tested."
    ],
    windowDays: 7,
    requiredParameters: [],
    resultFields: [...RESULT_FIELDS],
    ordering: [...ORDERING]
  };
}
