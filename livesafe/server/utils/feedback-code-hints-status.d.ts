export interface FeedbackCodeHintsStatusPayload {
  status: "inactive";
  api_surface: "api-response";
  read_only: true;
  code_hints_route_enabled: boolean;
  code_hints_registry_enabled: boolean;
  supported_components: string[];
  code_hints_fields: string[];
  allowed_operations: string[];
  blocked_operations: string[];
  public_claims_allowed: boolean;
  source_basis: string[];
  generated_at: string;
}

export interface CreateFeedbackCodeHintsStatusOptions {
  generatedAt?: string;
}

export function createFeedbackCodeHintsStatusPayload(
  options?: CreateFeedbackCodeHintsStatusOptions
): FeedbackCodeHintsStatusPayload;

export function sendFeedbackCodeHintsStatusResponse(
  req: unknown,
  res: {
    status(code: number): { json(payload: FeedbackCodeHintsStatusPayload): unknown };
  },
  options?: CreateFeedbackCodeHintsStatusOptions
): unknown;
