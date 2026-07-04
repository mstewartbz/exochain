import {
  evaluateExochainBoundary,
  EXOCHAIN_RECORD_TYPES_ALLOWED_AFTER_VERIFIED_ADAPTER,
  LIVESAFE_PRODUCT_SURFACES,
  LIVE_SAFE_SURFACE,
  requireAdjacentSurface
} from "../src/exochain-boundary.js";

describe("EXOCHAIN boundary evaluator", () => {
  it("allows an adjacent surface with no runtime trust claim and no core access", () => {
    const decision = evaluateExochainBoundary({
      classification: "adjacent-surface",
      adapterState: "not-wired",
      exochainResponse: "not-called",
      claimsExochainTrust: false,
      readsExochainCoreState: false,
      writesExochainCoreState: false,
      storesRawSensitiveDataOnChain: false
    });

    expect(decision).toEqual({
      allowed: true,
      reasons: [],
      requiredEvidence: []
    });
  });

  it("rejects EXOCHAIN trust claims without a verified adapter", () => {
    const decision = evaluateExochainBoundary({
      classification: "adjacent-surface",
      adapterState: "not-wired",
      exochainResponse: "not-called",
      claimsExochainTrust: true,
      readsExochainCoreState: false,
      writesExochainCoreState: false,
      storesRawSensitiveDataOnChain: false
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "EXOCHAIN trust claims require a verified runtime adapter."
    );
    expect(decision.requiredEvidence).toContain(
      "Adapter tests proving fail-closed behavior."
    );
  });

  it("fails closed when a verified adapter receives a non-permit response", () => {
    for (const exochainResponse of [
      "deny",
      "rejected",
      "timeout",
      "unavailable",
      "not-called",
      "stale",
      "revoked",
      "contradicted",
    ] as const) {
      const denied = evaluateExochainBoundary({
        classification: "core-runtime-adapter",
        adapterState: "verified",
        exochainResponse,
        claimsExochainTrust: true,
        readsExochainCoreState: true,
        writesExochainCoreState: false,
        storesRawSensitiveDataOnChain: false
      });

      expect(denied.allowed).toBe(false);
      expect(denied.reasons).toContain(
        "Verified adapters must fail closed unless EXOCHAIN permits."
      );
      expect(denied.requiredEvidence).toContain(
        "Denied, rejected, timeout, unavailable, not-called, stale, revoked, and contradicted-path regression tests."
      );
    }
  });

  it("rejects raw sensitive data on-chain even with a verified adapter", () => {
    const decision = evaluateExochainBoundary({
      classification: "core-runtime-adapter",
      adapterState: "verified",
      exochainResponse: "permit",
      claimsExochainTrust: true,
      readsExochainCoreState: true,
      writesExochainCoreState: true,
      storesRawSensitiveDataOnChain: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain("Raw sensitive data must remain off-chain.");
  });

  it("keeps LiveSafe classified as an adjacent surface", () => {
    expect(LIVE_SAFE_SURFACE).toMatchObject({
      name: "LiveSafe",
      owner: "bob-stewart",
      repository: "github.com/bob-stewart/livesafe",
      exochainRepository: "github.com/exochain/exochain",
      classification: "adjacent-surface",
      trustClaimsAllowed: false
    });

    expect(() => requireAdjacentSurface("adjacent-surface")).not.toThrow();
    expect(() => requireAdjacentSurface("exochain-core")).toThrow(
      "Expected adjacent-surface classification, got exochain-core."
    );
  });

  it("tracks the current LiveSafe surface vocabulary and allowed record types", () => {
    expect(Object.keys(LIVESAFE_PRODUCT_SURFACES).sort()).toEqual([
      "ambient",
      "inCaseOfEmergencyCard",
      "liveSafe",
      "vitalLock"
    ]);

    expect(EXOCHAIN_RECORD_TYPES_ALLOWED_AFTER_VERIFIED_ADAPTER).toEqual([
      "content-addressed-references",
      "commitments",
      "hashes",
      "policy-references",
      "access-logs",
      "custody-receipts"
    ]);
  });
});
