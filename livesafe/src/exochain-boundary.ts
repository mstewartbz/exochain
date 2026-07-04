export type SurfaceClassification =
  | "exochain-core"
  | "core-runtime-adapter"
  | "adjacent-surface"
  | "imported-evidence"
  | "third-party-vendor";

export type AdapterState = "not-wired" | "unverified" | "verified";

export type ExochainResponseState =
  | "permit"
  | "deny"
  | "rejected"
  | "timeout"
  | "unavailable"
  | "not-called"
  | "stale"
  | "revoked"
  | "contradicted";

export interface BoundaryRequest {
  classification: SurfaceClassification;
  adapterState: AdapterState;
  exochainResponse: ExochainResponseState;
  claimsExochainTrust: boolean;
  readsExochainCoreState: boolean;
  writesExochainCoreState: boolean;
  storesRawSensitiveDataOnChain: boolean;
}

export interface BoundaryDecision {
  allowed: boolean;
  reasons: string[];
  requiredEvidence: string[];
}

export const LIVE_SAFE_SURFACE = {
  name: "LiveSafe",
  owner: "bob-stewart",
  repository: "github.com/bob-stewart/livesafe",
  exochainRepository: "github.com/exochain/exochain",
  classification: "adjacent-surface",
  releaseStatus: "prototype",
  trustClaimsAllowed: false
} as const;

export const LIVESAFE_PRODUCT_SURFACES = {
  liveSafe: "safety-network",
  vitalLock: "protected-vault",
  inCaseOfEmergencyCard: "emergency-access-artifact",
  ambient: "always-on-context-layer"
} as const;

export const EXOCHAIN_RECORD_TYPES_ALLOWED_AFTER_VERIFIED_ADAPTER = [
  "content-addressed-references",
  "commitments",
  "hashes",
  "policy-references",
  "access-logs",
  "custody-receipts"
] as const;

export function evaluateExochainBoundary(
  request: BoundaryRequest
): BoundaryDecision {
  const reasons: string[] = [];
  const requiredEvidence = new Set<string>();

  if (request.classification === "exochain-core") {
    reasons.push("LiveSafe changes must not be classified as EXOCHAIN core.");
    requiredEvidence.add("Explicit Bob Stewart instruction authorizing core work.");
  }

  if (request.claimsExochainTrust && request.adapterState !== "verified") {
    reasons.push("EXOCHAIN trust claims require a verified runtime adapter.");
    requiredEvidence.add("Adapter tests proving fail-closed behavior.");
    requiredEvidence.add("Runtime path invoking the relevant EXOCHAIN core API.");
  }

  if (
    (request.readsExochainCoreState || request.writesExochainCoreState) &&
    request.adapterState !== "verified"
  ) {
    reasons.push("EXOCHAIN core state access requires a verified adapter.");
    requiredEvidence.add("Core API contract and permission boundary tests.");
  }

  if (
    request.adapterState === "verified" &&
    request.exochainResponse !== "permit"
  ) {
    reasons.push("Verified adapters must fail closed unless EXOCHAIN permits.");
    requiredEvidence.add(
      "Denied, rejected, timeout, unavailable, not-called, stale, revoked, and contradicted-path regression tests."
    );
  }

  if (request.storesRawSensitiveDataOnChain) {
    reasons.push("Raw sensitive data must remain off-chain.");
    requiredEvidence.add("Storage design showing only commitments or receipts on-chain.");
  }

  return {
    allowed: reasons.length === 0,
    reasons,
    requiredEvidence: [...requiredEvidence].sort()
  };
}

export function requireAdjacentSurface(
  classification: SurfaceClassification
): asserts classification is "adjacent-surface" {
  if (classification !== "adjacent-surface") {
    throw new Error(`Expected adjacent-surface classification, got ${classification}.`);
  }
}
