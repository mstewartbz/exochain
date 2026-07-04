"use strict";

const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");
const {
  exochainProductionTrustEvidence,
} = require("./exochain-production-trust-evidence");

const SOURCE_BASIS = [
  "docs/context/LIVESAFE_PRODUCTION_TRUST_ACTIVATION_GATES.md",
  "docs/context/LIVESAFE_TRUST_SIGNAL_VISUAL_LANGUAGE.md",
  "src/trust-signal.ts",
  "src/genesis-trust.ts",
  "server/utils/livesafe-exochain-adapter.js"
];

const NOT_VERIFIED_TOKEN = {
  state: "not-verified",
  badge_text: "AVC",
  icon: "lock-open",
  color: "red",
  css_class: "trust-signal trust-signal--red trust-signal--not-verified",
  glow_class: "trust-glow trust-glow--red",
  display_text: "THIS IS NOT YET VERIFIED",
  machine_state: "not_verified"
};

const VERIFIED_TOKEN = {
  state: "externally-verified",
  badge_text: "AVC",
  icon: "lock-check",
  color: "green",
  css_class: "trust-signal trust-signal--green trust-signal--externally-verified",
  glow_class: "trust-glow trust-glow--green",
  display_text: "VERIFIED",
  machine_state: "public_trust_claims_allowed"
};

function isProductionEvidenceVerified(productionTrustEvidence) {
  return (
    productionTrustEvidence?.evidence_state === "verified" &&
    productionTrustEvidence.production_health_verified === true &&
    productionTrustEvidence.production_ready_verified === true &&
    productionTrustEvidence.root_trust_bundle_verified === true
  );
}

function publicClaimsReason({
  productionEvidenceVerified,
  runtimeAdapterVerified,
  runtimePublicClaimsAllowed,
}) {
  if (!productionEvidenceVerified) {
    return "Public trust claims remain inactive until EXOCHAIN production evidence verifies.";
  }

  if (!runtimeAdapterVerified) {
    return "Public trust claims remain inactive because EXOCHAIN production evidence is verified but the LiveSafe runtime adapter remains unverified.";
  }

  if (!runtimePublicClaimsAllowed) {
    return "Public trust claims remain inactive because the LiveSafe runtime adapter has not allowed public trust output.";
  }

  return "EXOCHAIN production evidence and LiveSafe runtime adapter gates are verified.";
}

function createTrustStatusPayload({
  exochainConnected,
  version,
  uptimeSeconds,
  generatedAt,
  runtimeStatus,
  productionTrustEvidence
}) {
  const defaultRuntimeStatus = runtimeExochainAdapter.getRuntimeStatus();
  const resolvedRuntimeStatus = runtimeStatus
    ? {
        ...defaultRuntimeStatus,
        ...runtimeStatus,
        wrapped_operations:
          runtimeStatus.wrapped_operations ?? defaultRuntimeStatus.wrapped_operations,
      }
    : defaultRuntimeStatus;

  const resolvedProductionTrustEvidence =
    productionTrustEvidence || exochainProductionTrustEvidence;
  const runtimeAdapterVerified =
    resolvedRuntimeStatus.adapter_state === "verified";
  const productionEvidenceVerified = isProductionEvidenceVerified(
    resolvedProductionTrustEvidence,
  );
  const runtimePublicClaimsAllowed =
    resolvedRuntimeStatus.public_claims_allowed === true;
  const publicClaimsAllowed =
    productionEvidenceVerified &&
    runtimeAdapterVerified &&
    runtimePublicClaimsAllowed;
  const trustToken = publicClaimsAllowed ? VERIFIED_TOKEN : NOT_VERIFIED_TOKEN;

  return {
    ...trustToken,
    api_surface: "api-response",
    exochain_connected: Boolean(exochainConnected),
    verified_runtime_adapter: runtimeAdapterVerified,
    runtime_adapter_state: resolvedRuntimeStatus.adapter_state,
    adapter_surface_classification: resolvedRuntimeStatus.surface_classification,
    runtime_adapter_operations: resolvedRuntimeStatus.wrapped_operations,
    adapter_disablement_path: resolvedRuntimeStatus.disablement_path,
    exochain_production_evidence_state:
      resolvedProductionTrustEvidence.evidence_state,
    exochain_production_health_verified:
      resolvedProductionTrustEvidence.production_health_verified === true,
    exochain_production_ready_verified:
      resolvedProductionTrustEvidence.production_ready_verified === true,
    exochain_root_trust_bundle_verified:
      resolvedProductionTrustEvidence.root_trust_bundle_verified === true,
    exochain_root_trust_bundle_id:
      resolvedProductionTrustEvidence.root_trust_bundle_id || null,
    exochain_root_trust_ceremony_id:
      resolvedProductionTrustEvidence.root_trust_ceremony_id || null,
    exochain_root_trust_issuer_did:
      resolvedProductionTrustEvidence.root_trust_issuer_did || null,
    exochain_root_trust_verifier_commit:
      resolvedProductionTrustEvidence.verifier_commit || null,
    exochain_root_trust_verified_at:
      resolvedProductionTrustEvidence.verified_at || null,
    production_trust_observations:
      resolvedProductionTrustEvidence.non_blocking_observations || [],
    production_trust_reasons:
      resolvedProductionTrustEvidence.reasons || [],
    internal_proof_complete: productionEvidenceVerified,
    frost_genesis_complete: productionEvidenceVerified,
    public_claims_allowed: publicClaimsAllowed,
    public_claims_reason: publicClaimsReason({
      productionEvidenceVerified,
      runtimeAdapterVerified,
      runtimePublicClaimsAllowed,
    }),
    source_basis: [
      ...SOURCE_BASIS,
      "config/exochain-production-trust.json",
      "server/utils/exochain-production-trust-evidence.js",
    ],
    version,
    uptime_seconds: uptimeSeconds,
    generated_at: generatedAt ?? new Date().toISOString()
  };
}

function sendTrustStatusResponse(_req, res, options) {
  return res.status(200).json(createTrustStatusPayload(options));
}

module.exports = {
  createTrustStatusPayload,
  sendTrustStatusResponse
};
