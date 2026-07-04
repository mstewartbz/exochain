export interface AiHelpStatusOptions {
  helpAiEnabled?: boolean | string;
  feedbackWritesEnabled?: boolean | string;
  helpAiMandatedReporterEnabled?: boolean | string;
  feedbackAgentDispatchEnabled?: boolean | string;
  feedbackScreenshotsEnabled?: boolean | string;
  feedbackCodeHintsEnabled?: boolean | string;
  feedbackAgentTriggerStatuses?: string;
  helpAiSessionTtlHours?: number | string;
  helpAiReportIntervalMinutes?: number | string;
  helpAiUnansweredThreshold?: number | string;
  generatedAt?: string;
}

export interface AiHelpStatusPayload {
  status: "inactive";
  api_surface: "api-response";
  read_only: true;
  write_routes_enabled: false;
  help_ai_enabled: boolean;
  feedback_writes_enabled: boolean;
  help_ai_mandated_reporter_enabled: boolean;
  feedback_agent_dispatch_enabled: boolean;
  feedback_screenshots_enabled: boolean;
  feedback_code_hints_enabled: boolean;
  feedback_agent_trigger_statuses: string[];
  help_ai_session_ttl_hours: number;
  help_ai_report_interval_minutes: number;
  help_ai_unanswered_threshold: number;
  allowed_operations: string[];
  blocked_operations: string[];
  public_claims_allowed: false;
  source_basis: string[];
  generated_at: string;
}

export function createAiHelpStatusPayload(
  options?: AiHelpStatusOptions
): AiHelpStatusPayload;

export function sendAiHelpStatusResponse(
  req: unknown,
  res: {
    status(code: number): {
      json(payload: AiHelpStatusPayload): unknown;
    };
  },
  options?: AiHelpStatusOptions
): unknown;
