import {
  evaluateCivicSourceUse,
  evaluateIpDisclosure,
  LIVESAFE_CIVIC_SOURCE_REFERENCES,
  LIVESAFE_PROPRIETARY_ASSETS
} from "../src/ip-boundary.js";

describe("LiveSafe IP boundary evaluator", () => {
  it("keeps proprietary architecture out of public targets", () => {
    const decision = evaluateIpDisclosure({
      classification: "proprietary-internal",
      target: "public-repository",
      includesDetailedArchitecture: true,
      includesTransferArtifact: true,
      includesImplementationPrompt: true,
      includesSensitiveOperationalData: false,
      hasSourceProvenance: true,
      approvedForPublicRelease: false
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Proprietary internal IP cannot move to a public target."
    );
    expect(decision.requiredEvidence).toContain(
      "Owner-approved public-release classification."
    );
  });

  it("allows controlled private use when provenance is present and sensitive data is absent", () => {
    const decision = evaluateIpDisclosure({
      classification: "proprietary-internal",
      target: "controlled-agent-session",
      includesDetailedArchitecture: true,
      includesTransferArtifact: true,
      includesImplementationPrompt: true,
      includesSensitiveOperationalData: false,
      hasSourceProvenance: true,
      approvedForPublicRelease: false
    });

    expect(decision).toEqual({
      allowed: true,
      reasons: [],
      requiredEvidence: []
    });
  });

  it("denies IP movement without source provenance", () => {
    const decision = evaluateIpDisclosure({
      classification: "private-source-evidence",
      target: "private-repository",
      includesDetailedArchitecture: false,
      includesTransferArtifact: false,
      includesImplementationPrompt: false,
      includesSensitiveOperationalData: false,
      hasSourceProvenance: false,
      approvedForPublicRelease: false
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Project IP movements require source provenance."
    );
  });

  it("denies sensitive operational data in IP artifacts", () => {
    const decision = evaluateIpDisclosure({
      classification: "approved-public-summary",
      target: "private-customer-room",
      includesDetailedArchitecture: false,
      includesTransferArtifact: false,
      includesImplementationPrompt: false,
      includesSensitiveOperationalData: true,
      hasSourceProvenance: true,
      approvedForPublicRelease: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Sensitive operational data cannot be disclosed through IP artifacts."
    );
  });

  it("tracks the proprietary asset set by stable ids", () => {
    expect(LIVESAFE_PROPRIETARY_ASSETS).toEqual([
      "exo-legacy-transfer-package",
      "ice-card-foldable-packet-concept",
      "pace-social-contract-onboarding",
      "medical-jacket-custody-model",
      "phenotypical-genotypical-data-classification",
      "content-addressed-storage-entitlement-model",
      "ai-help-feedback-agent-system",
      "marketplace-template-entitlement-model",
      "frontline-free-family-plan-eligibility"
    ]);
  });

  it("distinguishes public-domain civic sources from proprietary implementation", () => {
    expect(LIVESAFE_CIVIC_SOURCE_REFERENCES).toEqual([
      {
        id: "us-constitution-preamble",
        phrase: "We the People",
        provenance: "United States Constitution Preamble",
        classification: "public-domain-civic-source"
      },
      {
        id: "gettysburg-address-civic-formula",
        phrase: "of the people, by the people, for the people",
        provenance: "Gettysburg Address",
        classification: "public-domain-civic-source"
      }
    ]);
  });

  it("denies treating public-domain civic text as proprietary IP", () => {
    const decision = evaluateCivicSourceUse({
      use: "private-doctrine",
      citesSourceProvenance: true,
      treatsPublicDomainTextAsProprietary: true,
      claimsGovernmentAuthority: false,
      claimsLegalEnforcementFromCivicSourceAlone: false,
      blendsWithProprietaryImplementation: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Public-domain civic source text cannot be classified as proprietary IP."
    );
  });

  it("denies government-authority or legal-force claims from civic language alone", () => {
    const decision = evaluateCivicSourceUse({
      use: "runtime-authority-claim",
      citesSourceProvenance: true,
      treatsPublicDomainTextAsProprietary: false,
      claimsGovernmentAuthority: true,
      claimsLegalEnforcementFromCivicSourceAlone: true,
      blendsWithProprietaryImplementation: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "LiveSafe cannot imply governmental authority from civic-source language."
    );
    expect(decision.reasons).toContain(
      "Civic-source language alone cannot be used as a legal-enforcement claim."
    );
    expect(decision.reasons).toContain(
      "Runtime authority claims must be grounded in verified code and policy, not civic rhetoric."
    );
  });
});
