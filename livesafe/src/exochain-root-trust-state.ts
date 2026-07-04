export type LiveSafeTrustClassification =
  | "not_verified"
  | "exochain_root_evidence_verified"
  | "livesafe_adapter_verified"
  | "public_trust_claims_allowed";

export interface ExochainRootEvidenceFacts {
  exoRootCratePresent: boolean;
  genesisCeremonyPolicyPresent: boolean;
  frostThresholdPolicyPresent: boolean;
  rootBundleVerificationPresent: boolean;
  rootSignatureVerificationPresent: boolean;
}

export interface LiveSafeTrustStateInput {
  rootEvidence: ExochainRootEvidenceFacts;
  adapterVerified: boolean;
  productionStatusVerified: boolean;
}

export interface LiveSafeTrustStateDecision {
  classification: LiveSafeTrustClassification;
  publicClaimsAllowed: boolean;
  evidenceGaps: string[];
}

function collectRootEvidenceGaps(
  rootEvidence: ExochainRootEvidenceFacts
): string[] {
  const evidenceGaps: string[] = [];

  if (!rootEvidence.exoRootCratePresent) {
    evidenceGaps.push("Missing EXOCHAIN exo-root crate evidence.");
  }
  if (!rootEvidence.genesisCeremonyPolicyPresent) {
    evidenceGaps.push("Missing EXOCHAIN root genesis ceremony policy evidence.");
  }
  if (!rootEvidence.frostThresholdPolicyPresent) {
    evidenceGaps.push("Missing EXOCHAIN 7-of-13 FROST threshold evidence.");
  }
  if (!rootEvidence.rootBundleVerificationPresent) {
    evidenceGaps.push("Missing EXOCHAIN root trust bundle verification evidence.");
  }
  if (!rootEvidence.rootSignatureVerificationPresent) {
    evidenceGaps.push("Missing EXOCHAIN root signature verification evidence.");
  }

  return evidenceGaps;
}

export function evaluateExochainRootTrustState(
  input: LiveSafeTrustStateInput
): LiveSafeTrustStateDecision {
  const evidenceGaps = collectRootEvidenceGaps(input.rootEvidence);
  const rootEvidenceVerified = evidenceGaps.length === 0;

  if (!rootEvidenceVerified) {
    return {
      classification: "not_verified",
      publicClaimsAllowed: false,
      evidenceGaps
    };
  }

  if (!input.adapterVerified) {
    return {
      classification: "exochain_root_evidence_verified",
      publicClaimsAllowed: false,
      evidenceGaps: []
    };
  }

  if (!input.productionStatusVerified) {
    return {
      classification: "livesafe_adapter_verified",
      publicClaimsAllowed: false,
      evidenceGaps: []
    };
  }

  return {
    classification: "public_trust_claims_allowed",
    publicClaimsAllowed: true,
    evidenceGaps: []
  };
}
