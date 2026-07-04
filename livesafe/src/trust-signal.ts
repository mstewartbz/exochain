export type TrustSignalState =
  | "not-verified"
  | "genesis-pending"
  | "internal-proof"
  | "externally-verified";

export type TrustSignalSurface =
  | "internal-console"
  | "private-review"
  | "customer-portal"
  | "public-website"
  | "printed-card"
  | "api-response";

export type TrustSignalColor = "red" | "yellow" | "blue" | "green";

export type TrustSignalDevice =
  | "mobile"
  | "tablet"
  | "desktop"
  | "print"
  | "api";

export type TrustSignalHolonContext =
  | "individual"
  | "family"
  | "pace-network"
  | "responder"
  | "organization"
  | "agent";

export type TrustSignalTextDirection = "ltr" | "rtl";

export interface TrustSignalToken {
  state: TrustSignalState;
  badge: "AVC";
  icon: "lock-open" | "lock-clock" | "shield-check" | "lock-check";
  color: TrustSignalColor;
  cssClass: string;
  glowClass: string;
  displayText: string;
  machineText: string;
  externalTrustClaimAllowed: boolean;
}

export interface TrustSignalOutputRequest {
  state: TrustSignalState;
  surface: TrustSignalSurface;
  includesTrustBearingClaim: boolean;
  hasAvcBadge: boolean;
  hasLockSymbol: boolean;
  hasColorTreatment: boolean;
  hasGlowTreatment: boolean;
  hasHumanReadableStatus: boolean;
  hasMachineReadableStatus: boolean;
  hasAccessibleLabel: boolean;
}

export interface TrustSignalOutputDecision {
  allowed: boolean;
  reasons: string[];
  requiredEvidence: string[];
}

export interface TrustSignalHomologationRequest {
  state: TrustSignalState;
  device: TrustSignalDevice;
  holonContext: TrustSignalHolonContext;
  localeTag: string;
  languageTag: string;
  jurisdictionCode: string;
  regionCode: string;
  scriptCode: string;
  textDirection: TrustSignalTextDirection;
  preservesMachineState: boolean;
  preservesDisplayMeaning: boolean;
  hasLocalizedStatusText: boolean;
  hasCulturalSymbolReview: boolean;
  hasNonColorOnlyStatus: boolean;
  supportsAssistiveTechnology: boolean;
  minTouchTargetPx: number;
  layoutStable: boolean;
}

export const TRUST_SIGNAL_TOKENS: Record<TrustSignalState, TrustSignalToken> = {
  "not-verified": {
    state: "not-verified",
    badge: "AVC",
    icon: "lock-open",
    color: "red",
    cssClass: "trust-signal trust-signal--red trust-signal--not-verified",
    glowClass: "trust-glow trust-glow--red",
    displayText: "THIS IS NOT YET VERIFIED",
    machineText: "not_verified",
    externalTrustClaimAllowed: false
  },
  "genesis-pending": {
    state: "genesis-pending",
    badge: "AVC",
    icon: "lock-clock",
    color: "yellow",
    cssClass: "trust-signal trust-signal--yellow trust-signal--genesis-pending",
    glowClass: "trust-glow trust-glow--yellow",
    displayText: "GENESIS VERIFICATION PENDING",
    machineText: "genesis_pending",
    externalTrustClaimAllowed: false
  },
  "internal-proof": {
    state: "internal-proof",
    badge: "AVC",
    icon: "shield-check",
    color: "blue",
    cssClass: "trust-signal trust-signal--blue trust-signal--internal-proof",
    glowClass: "trust-glow trust-glow--blue",
    displayText: "INTERNAL PROOF ONLY",
    machineText: "internal_proof_only",
    externalTrustClaimAllowed: false
  },
  "externally-verified": {
    state: "externally-verified",
    badge: "AVC",
    icon: "lock-check",
    color: "green",
    cssClass: "trust-signal trust-signal--green trust-signal--externally-verified",
    glowClass: "trust-glow trust-glow--green",
    displayText: "VERIFIED",
    machineText: "externally_verified",
    externalTrustClaimAllowed: true
  }
};

export const TRUST_SIGNAL_HOLON_CONTEXTS: TrustSignalHolonContext[] = [
  "individual",
  "family",
  "pace-network",
  "responder",
  "organization",
  "agent"
];

export const TRUST_SIGNAL_MODALITIES = {
  jurisdictional: ["country", "region", "subdivision", "legal-regime"],
  geographic: ["region", "locale", "script", "text-direction"],
  linguistic: ["language", "script", "terminology", "reading-level"],
  ethnographic: [
    "plain-language",
    "cultural-symbol-review",
    "non-color-only-status",
    "assistive-technology"
  ],
  device: ["mobile", "tablet", "desktop", "print", "api"],
  holonic: TRUST_SIGNAL_HOLON_CONTEXTS
} as const;

export const TRUST_SIGNAL_SUPPORTED_SCRIPTS = [
  "Arab",
  "Cyrl",
  "Deva",
  "Grek",
  "Hans",
  "Hant",
  "Hebr",
  "Hang",
  "Jpan",
  "Kana",
  "Kore",
  "Latn",
  "Thai"
] as const;

const publicSurfaces = new Set<TrustSignalSurface>([
  "customer-portal",
  "public-website",
  "printed-card",
  "api-response"
]);

const supportedScripts = new Set<string>(TRUST_SIGNAL_SUPPORTED_SCRIPTS);

function hasValue(value: string): boolean {
  return value.trim().length > 0;
}

export function evaluateTrustSignalOutput(
  request: TrustSignalOutputRequest
): TrustSignalOutputDecision {
  const reasons: string[] = [];
  const requiredEvidence = new Set<string>();
  const token = TRUST_SIGNAL_TOKENS[request.state];
  const publicTrustBearing =
    request.includesTrustBearingClaim && publicSurfaces.has(request.surface);

  if (publicTrustBearing && !request.hasAvcBadge) {
    reasons.push("Trust-bearing public output requires an AVC badge.");
    requiredEvidence.add("Rendered AVC badge adjacent to the trust-bearing claim.");
  }

  if (publicTrustBearing && !request.hasColorTreatment) {
    reasons.push("Trust-bearing public output requires colorized status treatment.");
    requiredEvidence.add(`Rendered ${token.color} trust-signal color class.`);
  }

  if (publicTrustBearing && !request.hasLockSymbol) {
    reasons.push("Trust-bearing public output requires a lock-style or shield-style symbol.");
    requiredEvidence.add("Rendered lock-style or shield-style trust symbol.");
  }

  if (publicTrustBearing && !request.hasGlowTreatment) {
    reasons.push("Trust-bearing public output requires CSS glow treatment.");
    requiredEvidence.add(`Rendered ${token.glowClass} treatment.`);
  }

  if (publicTrustBearing && !request.hasHumanReadableStatus) {
    reasons.push("Trust-bearing public output requires human-readable status text.");
    requiredEvidence.add(`Rendered status text: ${token.displayText}.`);
  }

  if (publicTrustBearing && !request.hasMachineReadableStatus) {
    reasons.push("Trust-bearing public output requires machine-readable status.");
    requiredEvidence.add(`Rendered machine state: ${token.machineText}.`);
  }

  if (publicTrustBearing && !request.hasAccessibleLabel) {
    reasons.push(
      "Trust-bearing public output requires an accessible label equivalent to the status text."
    );
    requiredEvidence.add("Accessible label bound to the rendered trust-state status.");
  }

  if (
    publicTrustBearing &&
    request.state === "not-verified" &&
    (!request.hasLockSymbol || !request.hasGlowTreatment)
  ) {
    reasons.push("Not-verified output must include a lock-style symbol and glow treatment.");
    requiredEvidence.add("Rendered lock symbol and red glow treatment.");
  }

  if (
    publicTrustBearing &&
    request.state !== "externally-verified" &&
    token.externalTrustClaimAllowed
  ) {
    reasons.push("Only externally verified trust state may allow external trust claims.");
    requiredEvidence.add("Token map proving non-green states deny external claims.");
  }

  return {
    allowed: reasons.length === 0,
    reasons,
    requiredEvidence: [...requiredEvidence].sort()
  };
}

export function evaluateTrustSignalHomologation(
  request: TrustSignalHomologationRequest
): TrustSignalOutputDecision {
  const reasons: string[] = [];
  const requiredEvidence = new Set<string>();

  if (!hasValue(request.jurisdictionCode)) {
    reasons.push("Trust homologation requires a jurisdiction code.");
    requiredEvidence.add("Jurisdiction mapping for the trust-state output.");
  }

  if (!hasValue(request.languageTag)) {
    reasons.push("Trust homologation requires a language tag.");
    requiredEvidence.add("BCP 47 language tag or equivalent localization record.");
  }

  if (!hasValue(request.localeTag) || !hasValue(request.regionCode)) {
    reasons.push("Trust homologation requires locale and region codes.");
    requiredEvidence.add("Locale and region mapping for the rendered trust output.");
  }

  if (!supportedScripts.has(request.scriptCode)) {
    reasons.push("Trust homologation requires a supported writing system.");
    requiredEvidence.add("Supported script code including Latin and relevant non-Latin scripts.");
  }

  if (!request.preservesMachineState) {
    reasons.push("Homologated trust output must preserve the canonical machine state.");
    requiredEvidence.add(`Canonical machine state for ${request.state}.`);
  }

  if (!request.preservesDisplayMeaning) {
    reasons.push("Homologated trust output must preserve the canonical display meaning.");
    requiredEvidence.add(`Localized text preserving ${TRUST_SIGNAL_TOKENS[request.state].displayText}.`);
  }

  if (!request.hasLocalizedStatusText) {
    reasons.push("Trust homologation requires localized status text.");
    requiredEvidence.add("Localized status phrase reviewed for the target language.");
  }

  if (!request.hasCulturalSymbolReview) {
    reasons.push("Trust symbols require cultural-symbol review for the target audience.");
    requiredEvidence.add("Cultural and ethnographic symbol review record.");
  }

  if (!request.hasNonColorOnlyStatus) {
    reasons.push("Trust state cannot rely on color alone.");
    requiredEvidence.add("Non-color cue such as icon, text, pattern, or machine state.");
  }

  if (!request.supportsAssistiveTechnology) {
    reasons.push("Trust homologation requires assistive-technology support.");
    requiredEvidence.add("Accessible label, screen-reader state, or equivalent API field.");
  }

  if (
    (request.device === "mobile" || request.device === "tablet") &&
    request.minTouchTargetPx < 44
  ) {
    reasons.push("Mobile and tablet trust controls require at least 44px touch targets.");
    requiredEvidence.add("Rendered mobile or tablet touch-target measurement.");
  }

  if (!request.layoutStable) {
    reasons.push("Holonic trust displays require stable layout across context levels.");
    requiredEvidence.add("Layout proof across individual, family, P.A.C.E., responder, organization, and agent contexts.");
  }

  return {
    allowed: reasons.length === 0,
    reasons,
    requiredEvidence: [...requiredEvidence].sort()
  };
}
