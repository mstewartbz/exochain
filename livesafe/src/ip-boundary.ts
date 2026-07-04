export type IpClassification =
  | "proprietary-internal"
  | "private-source-evidence"
  | "approved-public-summary"
  | "third-party-reference";

export type DisclosureTarget =
  | "private-repository"
  | "controlled-agent-session"
  | "private-customer-room"
  | "public-website"
  | "public-repository"
  | "public-issue-tracker"
  | "third-party-vendor";

export interface IpDisclosureRequest {
  classification: IpClassification;
  target: DisclosureTarget;
  includesDetailedArchitecture: boolean;
  includesTransferArtifact: boolean;
  includesImplementationPrompt: boolean;
  includesSensitiveOperationalData: boolean;
  hasSourceProvenance: boolean;
  approvedForPublicRelease: boolean;
}

export interface IpDisclosureDecision {
  allowed: boolean;
  reasons: string[];
  requiredEvidence: string[];
}

export type CivicSourceUse =
  | "private-doctrine"
  | "public-summary"
  | "product-copy"
  | "runtime-authority-claim";

export interface CivicSourceRequest {
  use: CivicSourceUse;
  citesSourceProvenance: boolean;
  treatsPublicDomainTextAsProprietary: boolean;
  claimsGovernmentAuthority: boolean;
  claimsLegalEnforcementFromCivicSourceAlone: boolean;
  blendsWithProprietaryImplementation: boolean;
}

const publicTargets = new Set<DisclosureTarget>([
  "public-website",
  "public-repository",
  "public-issue-tracker"
]);

export const LIVESAFE_PROPRIETARY_ASSETS = [
  "exo-legacy-transfer-package",
  "ice-card-foldable-packet-concept",
  "pace-social-contract-onboarding",
  "medical-jacket-custody-model",
  "phenotypical-genotypical-data-classification",
  "content-addressed-storage-entitlement-model",
  "ai-help-feedback-agent-system",
  "marketplace-template-entitlement-model",
  "frontline-free-family-plan-eligibility"
] as const;

export const LIVESAFE_CIVIC_SOURCE_REFERENCES = [
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
] as const;

export function evaluateIpDisclosure(
  request: IpDisclosureRequest
): IpDisclosureDecision {
  const reasons: string[] = [];
  const requiredEvidence = new Set<string>();

  if (!request.hasSourceProvenance) {
    reasons.push("Project IP movements require source provenance.");
    requiredEvidence.add("Source-basis record identifying origin and owner.");
  }

  if (request.includesSensitiveOperationalData) {
    reasons.push("Sensitive operational data cannot be disclosed through IP artifacts.");
    requiredEvidence.add("Redaction review proving only metadata or approved summaries remain.");
  }

  if (
    request.classification === "proprietary-internal" &&
    publicTargets.has(request.target)
  ) {
    reasons.push("Proprietary internal IP cannot move to a public target.");
    requiredEvidence.add("Owner-approved public-release classification.");
  }

  if (
    request.classification === "private-source-evidence" &&
    publicTargets.has(request.target)
  ) {
    reasons.push("Private source evidence cannot move to a public target.");
    requiredEvidence.add("Derived public summary with proprietary details removed.");
  }

  if (
    publicTargets.has(request.target) &&
    (request.includesDetailedArchitecture ||
      request.includesTransferArtifact ||
      request.includesImplementationPrompt) &&
    !request.approvedForPublicRelease
  ) {
    reasons.push("Detailed architecture, transfer artifacts, and build prompts need explicit public-release approval.");
    requiredEvidence.add("Approval record for the exact disclosed artifact.");
  }

  return {
    allowed: reasons.length === 0,
    reasons,
    requiredEvidence: [...requiredEvidence].sort()
  };
}

export function evaluateCivicSourceUse(
  request: CivicSourceRequest
): IpDisclosureDecision {
  const reasons: string[] = [];
  const requiredEvidence = new Set<string>();

  if (!request.citesSourceProvenance) {
    reasons.push("Civic-source use requires exact source provenance.");
    requiredEvidence.add("Citation distinguishing Constitution text from later civic doctrine.");
  }

  if (request.treatsPublicDomainTextAsProprietary) {
    reasons.push("Public-domain civic source text cannot be classified as proprietary IP.");
    requiredEvidence.add("Separation between public-domain source and proprietary implementation.");
  }

  if (request.claimsGovernmentAuthority) {
    reasons.push("LiveSafe cannot imply governmental authority from civic-source language.");
    requiredEvidence.add("Review proving the copy does not claim state action or official status.");
  }

  if (request.claimsLegalEnforcementFromCivicSourceAlone) {
    reasons.push("Civic-source language alone cannot be used as a legal-enforcement claim.");
    requiredEvidence.add("Verified legal, policy, contract, or runtime authority path.");
  }

  if (
    request.use === "runtime-authority-claim" &&
    request.blendsWithProprietaryImplementation
  ) {
    reasons.push("Runtime authority claims must be grounded in verified code and policy, not civic rhetoric.");
    requiredEvidence.add("Runtime tests and policy evidence for the claimed authority.");
  }

  return {
    allowed: reasons.length === 0,
    reasons,
    requiredEvidence: [...requiredEvidence].sort()
  };
}
