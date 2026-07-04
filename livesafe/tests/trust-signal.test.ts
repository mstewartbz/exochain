import {
  evaluateTrustSignalOutput,
  TRUST_SIGNAL_TOKENS
} from "../src/trust-signal.js";

describe("LiveSafe outward trust signal system", () => {
  it("renders not-verified output as a red AVC badge with unmistakable language", () => {
    expect(TRUST_SIGNAL_TOKENS["not-verified"]).toEqual({
      state: "not-verified",
      badge: "AVC",
      icon: "lock-open",
      color: "red",
      cssClass: "trust-signal trust-signal--red trust-signal--not-verified",
      glowClass: "trust-glow trust-glow--red",
      displayText: "THIS IS NOT YET VERIFIED",
      machineText: "not_verified",
      externalTrustClaimAllowed: false
    });
  });

  it("defines the full red yellow blue green trust-state palette", () => {
    expect(Object.keys(TRUST_SIGNAL_TOKENS)).toEqual([
      "not-verified",
      "genesis-pending",
      "internal-proof",
      "externally-verified"
    ]);
    expect(Object.values(TRUST_SIGNAL_TOKENS).map(token => token.color)).toEqual([
      "red",
      "yellow",
      "blue",
      "green"
    ]);
  });

  it("requires symbolic, textual, visual, and machine-readable status on public output", () => {
    const decision = evaluateTrustSignalOutput({
      state: "not-verified",
      surface: "public-website",
      includesTrustBearingClaim: true,
      hasAvcBadge: true,
      hasLockSymbol: true,
      hasColorTreatment: true,
      hasGlowTreatment: true,
      hasHumanReadableStatus: true,
      hasMachineReadableStatus: true,
      hasAccessibleLabel: true
    });

    expect(decision).toEqual({
      allowed: true,
      reasons: [],
      requiredEvidence: []
    });
  });

  it("denies public trust-bearing output without the visible AVC badge", () => {
    const decision = evaluateTrustSignalOutput({
      state: "genesis-pending",
      surface: "public-website",
      includesTrustBearingClaim: true,
      hasAvcBadge: false,
      hasLockSymbol: true,
      hasColorTreatment: true,
      hasGlowTreatment: true,
      hasHumanReadableStatus: true,
      hasMachineReadableStatus: true,
      hasAccessibleLabel: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Trust-bearing public output requires an AVC badge."
    );
  });

  it("denies public not-verified output if the state is not visually unmistakable", () => {
    const decision = evaluateTrustSignalOutput({
      state: "not-verified",
      surface: "customer-portal",
      includesTrustBearingClaim: true,
      hasAvcBadge: true,
      hasLockSymbol: false,
      hasColorTreatment: true,
      hasGlowTreatment: false,
      hasHumanReadableStatus: true,
      hasMachineReadableStatus: true,
      hasAccessibleLabel: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Not-verified output must include a lock-style symbol and glow treatment."
    );
  });

  it("allows green external verification only when the output carries the full signal", () => {
    const decision = evaluateTrustSignalOutput({
      state: "externally-verified",
      surface: "customer-portal",
      includesTrustBearingClaim: true,
      hasAvcBadge: true,
      hasLockSymbol: true,
      hasColorTreatment: true,
      hasGlowTreatment: true,
      hasHumanReadableStatus: true,
      hasMachineReadableStatus: true,
      hasAccessibleLabel: true
    });

    expect(decision.allowed).toBe(true);
    expect(TRUST_SIGNAL_TOKENS["externally-verified"].externalTrustClaimAllowed).toBe(true);
  });

  it("denies any trust-bearing public output that drops the required symbol, glow, or accessible label", () => {
    const decision = evaluateTrustSignalOutput({
      state: "externally-verified",
      surface: "api-response",
      includesTrustBearingClaim: true,
      hasAvcBadge: true,
      hasLockSymbol: false,
      hasColorTreatment: true,
      hasGlowTreatment: false,
      hasHumanReadableStatus: true,
      hasMachineReadableStatus: true,
      hasAccessibleLabel: false
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Trust-bearing public output requires a lock-style or shield-style symbol."
    );
    expect(decision.reasons).toContain(
      "Trust-bearing public output requires CSS glow treatment."
    );
    expect(decision.reasons).toContain(
      "Trust-bearing public output requires an accessible label equivalent to the status text."
    );
  });
});
