import { evaluateExochainRootTrustState } from "../src/exochain-root-trust-state.js";

describe("LiveSafe EXOCHAIN root trust-state ladder", () => {
  const verifiedRootEvidence = {
    exoRootCratePresent: true,
    genesisCeremonyPolicyPresent: true,
    frostThresholdPolicyPresent: true,
    rootBundleVerificationPresent: true,
    rootSignatureVerificationPresent: true
  } as const;

  it("holds the repo in not_verified when root evidence is incomplete", () => {
    expect(
      evaluateExochainRootTrustState({
        rootEvidence: {
          ...verifiedRootEvidence,
          rootBundleVerificationPresent: false
        },
        adapterVerified: false,
        productionStatusVerified: false
      })
    ).toEqual({
      classification: "not_verified",
      publicClaimsAllowed: false,
      evidenceGaps: ["Missing EXOCHAIN root trust bundle verification evidence."]
    });
  });

  it("promotes to exochain_root_evidence_verified without allowing public claims", () => {
    expect(
      evaluateExochainRootTrustState({
        rootEvidence: verifiedRootEvidence,
        adapterVerified: false,
        productionStatusVerified: false
      })
    ).toEqual({
      classification: "exochain_root_evidence_verified",
      publicClaimsAllowed: false,
      evidenceGaps: []
    });
  });

  it("keeps public claims blocked when the adapter is verified but production status is not", () => {
    expect(
      evaluateExochainRootTrustState({
        rootEvidence: verifiedRootEvidence,
        adapterVerified: true,
        productionStatusVerified: false
      })
    ).toEqual({
      classification: "livesafe_adapter_verified",
      publicClaimsAllowed: false,
      evidenceGaps: []
    });
  });

  it("allows public claims only after root evidence, adapter verification, and production status verification all pass", () => {
    expect(
      evaluateExochainRootTrustState({
        rootEvidence: verifiedRootEvidence,
        adapterVerified: true,
        productionStatusVerified: true
      })
    ).toEqual({
      classification: "public_trust_claims_allowed",
      publicClaimsAllowed: true,
      evidenceGaps: []
    });
  });
});
