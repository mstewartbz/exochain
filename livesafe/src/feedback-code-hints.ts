const SAFE_TOKEN_PATTERN = /^[A-Za-z0-9:_-]+$/;
const SAFE_REPO_PATH_PATTERN = /^(?:[A-Za-z0-9_-]+\/)*[A-Za-z0-9_.-]+$/;

export const feedbackCodeHintRegistry = {
  "onboarding-wizard": {
    targetType: "onboarding-step",
    description: "Account setup and next-best-action onboarding surfaces."
  },
  "pace-invite-flow": {
    targetType: "pace-contact",
    description: "P.A.C.E. invitation, acceptance, and replacement surfaces."
  },
  "ice-card-generator": {
    targetType: "ice-card",
    description: "ICE card packet generation and print surfaces."
  },
  "qr-activation": {
    targetType: "qr-activation",
    description: "QR activation and responder landing surfaces."
  },
  "responder-view": {
    targetType: "responder-view",
    description: "Responder-only emergency subset access surfaces."
  },
  "emergency-profile-editor": {
    targetType: "emergency-profile",
    description: "Emergency profile editing and release-bound field surfaces."
  },
  "medical-jacket": {
    targetType: "medical-jacket",
    description: "Medical jacket custody and record-class surfaces."
  },
  "consent-controls": {
    targetType: "consent-control",
    description: "Consent and revocation control surfaces."
  },
  "marketplace-template-config": {
    targetType: "marketplace-template",
    description: "Marketplace template selection and configuration surfaces."
  },
  "entitlement-plan-selector": {
    targetType: "entitlement-plan",
    description: "Plan, trial, gift, and capability selection surfaces."
  },
  "frontline-eligibility": {
    targetType: "frontline-eligibility",
    description: "Frontline cohort application and verification surfaces."
  },
  "trust-state-banner": {
    targetType: "trust-state",
    description: "Trust posture and EXOCHAIN-adjacent boundary surfaces."
  }
} as const;

export const feedbackCodeHintComponents = Object.keys(
  feedbackCodeHintRegistry
) as FeedbackCodeHintComponent[];

export const feedbackCodeHintFields = [
  "service",
  "filePaths",
  "specRef",
  "storageKeys",
  "apiOperation"
] as const;

export type FeedbackCodeHintComponent = keyof typeof feedbackCodeHintRegistry;

export interface FeedbackCodeHintsInput {
  service?: string;
  filePaths?: string[];
  specRef?: string;
  storageKeys?: string[];
  apiOperation?: string;
}

export interface FeedbackCodeHints {
  component: FeedbackCodeHintComponent;
  service?: string;
  filePaths?: string[];
  specRef?: string;
  storageKeys?: string[];
  apiOperation?: string;
}

export function buildFeedbackCodeHints(
  component: string,
  input: FeedbackCodeHintsInput
): FeedbackCodeHints {
  if (!isFeedbackCodeHintComponent(component)) {
    throw new Error(`Unsupported feedback code-hint component: ${component}.`);
  }

  const hints: FeedbackCodeHints = {
    component
  };

  if (input.service !== undefined) {
    hints.service = normalizeToken("service", input.service);
  }

  if (input.filePaths !== undefined) {
    hints.filePaths = normalizeRepoPaths("filePaths", input.filePaths);
  }

  if (input.specRef !== undefined) {
    hints.specRef = normalizeRepoPath("specRef", input.specRef);
  }

  if (input.storageKeys !== undefined) {
    hints.storageKeys = normalizeTokenList("storageKeys", input.storageKeys);
  }

  if (input.apiOperation !== undefined) {
    hints.apiOperation = normalizeToken("apiOperation", input.apiOperation);
  }

  return hints;
}

export function isFeedbackCodeHintComponent(
  value: string
): value is FeedbackCodeHintComponent {
  return value in feedbackCodeHintRegistry;
}

function normalizeToken(
  field: "service" | "apiOperation",
  value: string
): string {
  if (!SAFE_TOKEN_PATTERN.test(value)) {
    throw new Error(
      `feedback code-hint ${field} must match /^[A-Za-z0-9:_-]+$/.`
    );
  }

  return value;
}

function normalizeTokenList(field: "storageKeys", values: string[]): string[] {
  if (values.length === 0) {
    throw new Error(`feedback code-hint ${field} must not be empty.`);
  }

  for (const value of values) {
    if (!SAFE_TOKEN_PATTERN.test(value)) {
      throw new Error(
        `feedback code-hint ${field} entries must match /^[A-Za-z0-9:_-]+$/.`
      );
    }
  }

  return [...values];
}

function normalizeRepoPaths(
  field: "filePaths",
  values: string[]
): string[] {
  if (values.length === 0) {
    throw new Error(`feedback code-hint ${field} must not be empty.`);
  }

  return values.map((value) => normalizeRepoPath(field, value));
}

function normalizeRepoPath(field: "filePaths" | "specRef", value: string): string {
  if (
    value.startsWith("/") ||
    value.includes("..") ||
    !SAFE_REPO_PATH_PATTERN.test(value)
  ) {
    throw new Error(
      "feedback code-hint filePaths entries must stay within the LiveSafe repo and cannot traverse directories."
    );
  }

  return value;
}
