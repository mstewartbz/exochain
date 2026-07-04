const SESSION_ID_PATTERN = /^[A-Za-z0-9_-]+$/;

const TRANSCRIPT_RESULT_FIELDS = [
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
] as const;

const MESSAGE_FIELDS = ["role", "text", "timestamp"] as const;

const ACTIVE_SESSION_INDEX_FIELDS = [
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
] as const;

export const aiHelpSessionTranscriptQueryOperations = [
  "query-ai-help-session-transcript",
  "query-ai-help-active-session-index"
] as const;

export type AiHelpSessionTranscriptQueryOperation =
  (typeof aiHelpSessionTranscriptQueryOperations)[number];

interface BaseAiHelpSessionTranscriptQueryContract {
  operation: AiHelpSessionTranscriptQueryOperation;
  readOnly: true;
  routeEnabled: false;
  executionAllowed: false;
  blockers: readonly [
    "AI-help session-transcript query routes remain disabled until a backend is selected and tested."
  ];
  windowDays: 7;
}

export interface AiHelpSessionTranscriptQueryContractInput {
  operation: string;
  sessionId?: string;
  windowDays?: number;
}

export interface AiHelpSessionTranscriptLookupContract
  extends BaseAiHelpSessionTranscriptQueryContract {
  operation: "query-ai-help-session-transcript";
  requiredParameters: readonly ["sessionId"];
  resultFields: readonly (typeof TRANSCRIPT_RESULT_FIELDS)[number][];
  messageFields: readonly (typeof MESSAGE_FIELDS)[number][];
  sessionId: string;
}

export interface AiHelpActiveSessionIndexContract
  extends BaseAiHelpSessionTranscriptQueryContract {
  operation: "query-ai-help-active-session-index";
  requiredParameters: readonly [];
  resultFields: readonly (typeof ACTIVE_SESSION_INDEX_FIELDS)[number][];
  activeSessionIndexLimit: 50;
}

export type AiHelpSessionTranscriptQueryContract =
  | AiHelpSessionTranscriptLookupContract
  | AiHelpActiveSessionIndexContract;

export function isAiHelpSessionTranscriptQueryOperation(
  value: string
): value is AiHelpSessionTranscriptQueryOperation {
  return aiHelpSessionTranscriptQueryOperations.includes(
    value as AiHelpSessionTranscriptQueryOperation
  );
}

export function buildAiHelpSessionTranscriptQueryContract(
  input: AiHelpSessionTranscriptQueryContractInput
): AiHelpSessionTranscriptQueryContract {
  if (!isAiHelpSessionTranscriptQueryOperation(input.operation)) {
    throw new Error(
      `Unsupported AI-help session-transcript query operation: ${input.operation}.`
    );
  }

  if (input.operation === "query-ai-help-session-transcript") {
    if (input.windowDays !== undefined) {
      throw new Error(
        "query-ai-help-session-transcript does not accept windowDays overrides."
      );
    }

    if (
      input.sessionId === undefined ||
      !SESSION_ID_PATTERN.test(input.sessionId)
    ) {
      throw new Error(
        "query-ai-help-session-transcript requires a sessionId that matches /^[A-Za-z0-9_-]+$/."
      );
    }

    return {
      operation: input.operation,
      readOnly: true,
      routeEnabled: false,
      executionAllowed: false,
      blockers: [
        "AI-help session-transcript query routes remain disabled until a backend is selected and tested."
      ],
      windowDays: 7,
      requiredParameters: ["sessionId"],
      resultFields: [...TRANSCRIPT_RESULT_FIELDS],
      messageFields: [...MESSAGE_FIELDS],
      sessionId: input.sessionId
    };
  }

  if (input.sessionId !== undefined || input.windowDays !== undefined) {
    throw new Error(
      "query-ai-help-active-session-index does not accept sessionId or windowDays overrides."
    );
  }

  return {
    operation: input.operation,
    readOnly: true,
    routeEnabled: false,
    executionAllowed: false,
    blockers: [
      "AI-help session-transcript query routes remain disabled until a backend is selected and tested."
    ],
    windowDays: 7,
    requiredParameters: [],
    resultFields: [...ACTIVE_SESSION_INDEX_FIELDS],
    activeSessionIndexLimit: 50
  };
}
