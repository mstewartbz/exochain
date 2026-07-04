export type GenesisTrustSource =
  | "bob-direction"
  | "exoforge"
  | "verified-exochain-runtime"
  | "third-party";

export type GenesisTrustUse =
  | "development-planning"
  | "implementation"
  | "internal-validation"
  | "external-trust-signal"
  | "customer-runtime-claim";

export type GenesisAudience =
  | "internal-development"
  | "private-review"
  | "customer"
  | "public";

export type FrostKeygenCeremonyState =
  | "not-started"
  | "scheduled"
  | "in-progress"
  | "completed";

export interface GenesisTrustRequest {
  source: GenesisTrustSource;
  use: GenesisTrustUse;
  audience: GenesisAudience;
  sourceProvenanceRecorded: boolean;
  frostKeygenCeremony: FrostKeygenCeremonyState;
  frostThreshold: number;
  frostParticipants: number;
  internalProofComplete: boolean;
  verifiedRuntimeAdapter: boolean;
  signalsTrustExternally: boolean;
}

export interface GenesisTrustDecision {
  allowed: boolean;
  reasons: string[];
  requiredEvidence: string[];
}

export const FROST_GENESIS_CEREMONY = {
  scheme: "FROST",
  threshold: 7,
  participants: 13,
  status: "scheduled-this-week",
  externalTrustSignalAllowed: false
} as const;

const externalUses = new Set<GenesisTrustUse>([
  "external-trust-signal",
  "customer-runtime-claim"
]);

const externalAudiences = new Set<GenesisAudience>([
  "customer",
  "public"
]);

function isInternalDevelopmentUse(request: GenesisTrustRequest): boolean {
  return (
    request.audience === "internal-development" &&
    !request.signalsTrustExternally &&
    (request.use === "development-planning" ||
      request.use === "implementation" ||
      request.use === "internal-validation")
  );
}

function hasExactFrostGenesisProfile(request: GenesisTrustRequest): boolean {
  return (
    request.frostThreshold === FROST_GENESIS_CEREMONY.threshold &&
    request.frostParticipants === FROST_GENESIS_CEREMONY.participants
  );
}

export function evaluateGenesisTrust(
  request: GenesisTrustRequest
): GenesisTrustDecision {
  const reasons: string[] = [];
  const requiredEvidence = new Set<string>();

  if (!request.sourceProvenanceRecorded) {
    reasons.push("Development trust requires source provenance.");
    requiredEvidence.add("Source record naming Bob direction, ExoForge output, or verified runtime evidence.");
  }

  if (request.source === "third-party" && isInternalDevelopmentUse(request)) {
    reasons.push("Third-party sources cannot be trusted for internal development without classification.");
    requiredEvidence.add("Repository intake, license review, and IP classification.");
  }

  const externallyVisible =
    request.signalsTrustExternally ||
    externalUses.has(request.use) ||
    externalAudiences.has(request.audience);

  if (externallyVisible) {
    if (!request.internalProofComplete) {
      reasons.push("External trust signaling requires completed internal proof.");
      requiredEvidence.add("Internal proof gate report.");
    }

    if (request.frostKeygenCeremony !== "completed") {
      reasons.push("External trust signaling requires the 7-of-13 FROST keygen ceremony to be complete.");
      requiredEvidence.add("FROST keygen ceremony transcript and participant attestations.");
    }

    if (!hasExactFrostGenesisProfile(request)) {
      reasons.push("External trust signaling requires the exact 7-of-13 FROST ceremony profile.");
      requiredEvidence.add("Ceremony profile proving threshold 7 and participant count 13.");
    }

    if (!request.verifiedRuntimeAdapter) {
      reasons.push("External trust signaling requires a verified runtime adapter.");
      requiredEvidence.add("Runtime adapter tests proving fail-closed EXOCHAIN behavior.");
    }
  }

  return {
    allowed: reasons.length === 0,
    reasons,
    requiredEvidence: [...requiredEvidence].sort()
  };
}
