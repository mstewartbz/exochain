import {
  evaluateGenesisTrust,
  FROST_GENESIS_CEREMONY
} from "../src/genesis-trust.js";

describe("LiveSafe genesis development trust", () => {
  it("allows ExoForge for internal development during genesis", () => {
    const decision = evaluateGenesisTrust({
      source: "exoforge",
      use: "implementation",
      audience: "internal-development",
      sourceProvenanceRecorded: true,
      frostKeygenCeremony: "scheduled",
      frostThreshold: 7,
      frostParticipants: 13,
      internalProofComplete: false,
      verifiedRuntimeAdapter: false,
      signalsTrustExternally: false
    });

    expect(decision).toEqual({
      allowed: true,
      reasons: [],
      requiredEvidence: []
    });
  });

  it("denies external trust signaling before internal proof and FROST completion", () => {
    const decision = evaluateGenesisTrust({
      source: "exoforge",
      use: "external-trust-signal",
      audience: "public",
      sourceProvenanceRecorded: true,
      frostKeygenCeremony: "scheduled",
      frostThreshold: 7,
      frostParticipants: 13,
      internalProofComplete: false,
      verifiedRuntimeAdapter: false,
      signalsTrustExternally: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "External trust signaling requires completed internal proof."
    );
    expect(decision.reasons).toContain(
      "External trust signaling requires the 7-of-13 FROST keygen ceremony to be complete."
    );
    expect(decision.reasons).toContain(
      "External trust signaling requires a verified runtime adapter."
    );
  });

  it("requires the exact 7-of-13 FROST threshold before external signaling", () => {
    const decision = evaluateGenesisTrust({
      source: "verified-exochain-runtime",
      use: "external-trust-signal",
      audience: "public",
      sourceProvenanceRecorded: true,
      frostKeygenCeremony: "completed",
      frostThreshold: 6,
      frostParticipants: 13,
      internalProofComplete: true,
      verifiedRuntimeAdapter: true,
      signalsTrustExternally: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "External trust signaling requires the exact 7-of-13 FROST ceremony profile."
    );
  });

  it("denies development trust when source provenance is absent", () => {
    const decision = evaluateGenesisTrust({
      source: "exoforge",
      use: "development-planning",
      audience: "internal-development",
      sourceProvenanceRecorded: false,
      frostKeygenCeremony: "scheduled",
      frostThreshold: 7,
      frostParticipants: 13,
      internalProofComplete: false,
      verifiedRuntimeAdapter: false,
      signalsTrustExternally: false
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Development trust requires source provenance."
    );
  });

  it("records the genesis FROST profile as 7 of 13", () => {
    expect(FROST_GENESIS_CEREMONY).toEqual({
      scheme: "FROST",
      threshold: 7,
      participants: 13,
      status: "scheduled-this-week",
      externalTrustSignalAllowed: false
    });
  });
});
