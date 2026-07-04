export interface FeedbackBoardStatusPayload {
  status: "inactive";
  api_surface: "api-response";
  read_only: true;
  backend_selected: boolean;
  board_query_routes_enabled: boolean;
  feedback_write_routes_enabled: boolean;
  query_operations: string[];
  allowed_operations: string[];
  blocked_operations: string[];
  public_claims_allowed: boolean;
  source_basis: string[];
  generated_at: string;
}

export interface CreateFeedbackBoardStatusOptions {
  generatedAt?: string;
}

export function createFeedbackBoardStatusPayload(
  options?: CreateFeedbackBoardStatusOptions
): FeedbackBoardStatusPayload;

export function sendFeedbackBoardStatusResponse(
  req: unknown,
  res: { status(code: number): { json(payload: FeedbackBoardStatusPayload): unknown } },
  options?: CreateFeedbackBoardStatusOptions
): unknown;
