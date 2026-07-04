"use strict";

const exochainProductionTrustConfig = require("../../config/exochain-production-trust.json");

const DID_PATTERN = /^did:exo:[A-Za-z0-9._:-]+$/;
const SHA256_HEX_PATTERN = /^[a-f0-9]{64}$/;

function hasOkStatus(payload) {
  return Boolean(payload) && payload.status === "ok";
}

function isHttpsUrl(value) {
  return typeof value === "string" && value.startsWith("https://");
}

function hasSevenOfThirteenSignerPolicy(bundle) {
  if (!bundle || bundle.threshold !== 7 || bundle.maxSigners !== 13) {
    return false;
  }

  if (!Array.isArray(bundle.signerIds) || bundle.signerIds.length < bundle.threshold) {
    return false;
  }

  const uniqueSignerIds = new Set(bundle.signerIds);
  return (
    uniqueSignerIds.size >= bundle.threshold &&
    bundle.signerIds.every((id) => Number.isInteger(id) && id >= 1 && id <= bundle.maxSigners)
  );
}

function sentinelObservationKey(check) {
  return `production_sentinel_${String(check || "")
    .replace(/([a-z0-9])([A-Z])/g, "$1_$2")
    .replace(/[^a-zA-Z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "")
    .toLowerCase()}_below_bft_minimum`;
}

function evaluateSentinels({ sentinels = [], policy = {} }) {
  const reasons = [];
  const nonBlockingObservations = [];
  const byCheck = new Map(
    sentinels
      .filter((sentinel) => sentinel && typeof sentinel.check === "string")
      .map((sentinel) => [sentinel.check, sentinel]),
  );

  for (const requiredCheck of policy.requiredHealthy || []) {
    const sentinel = byCheck.get(requiredCheck);
    if (!sentinel || sentinel.healthy !== true) {
      reasons.push(`EXOCHAIN production sentinel ${requiredCheck} is not healthy.`);
    }
  }

  for (const observedCheck of policy.nonBlockingObservations || []) {
    const sentinel = byCheck.get(observedCheck);
    if (sentinel && sentinel.healthy !== true) {
      nonBlockingObservations.push(sentinelObservationKey(observedCheck));
    }
  }

  return { reasons, nonBlockingObservations };
}

function evaluateBundle(config) {
  const bundle = config.rootTrustBundle || {};
  const verification = config.verification || {};
  const reasons = [];

  if (verification.status !== "verified" || verification.result?.verified !== true) {
    reasons.push("EXOCHAIN root trust bundle verification is not confirmed.");
  }

  if (!SHA256_HEX_PATTERN.test(bundle.bundleIdHex || "")) {
    reasons.push("EXOCHAIN root trust bundle id is missing or malformed.");
  }

  if (!SHA256_HEX_PATTERN.test(bundle.bundleSha256 || "")) {
    reasons.push("EXOCHAIN root trust bundle SHA-256 is missing or malformed.");
  }

  if (bundle.ceremonyId !== "avc-exo-ceremony-2026") {
    reasons.push("EXOCHAIN root trust ceremony id does not match the expected AVC ceremony.");
  }

  if (!hasSevenOfThirteenSignerPolicy(bundle)) {
    reasons.push("EXOCHAIN root trust bundle does not carry a verified 7-of-13 signer policy.");
  }

  if (!DID_PATTERN.test(bundle.issuerDid || "")) {
    reasons.push("EXOCHAIN root trust issuer DID is missing or malformed.");
  }

  return {
    verified: reasons.length === 0,
    reasons,
  };
}

function evaluateExochainProductionTrustEvidence({
  config = exochainProductionTrustConfig,
  health = config.latestProductionProbe?.health,
  ready = config.latestProductionProbe?.ready,
  sentinels = config.latestProductionProbe?.sentinels || [],
} = {}) {
  const reasons = [];

  if (!isHttpsUrl(config.production?.baseUrl)) {
    reasons.push("EXOCHAIN production base URL must be HTTPS.");
  }

  const productionHealthVerified = hasOkStatus(health);
  const productionReadyVerified = hasOkStatus(ready);

  if (!productionHealthVerified) {
    reasons.push("EXOCHAIN production health probe did not return ok.");
  }

  if (!productionReadyVerified) {
    reasons.push("EXOCHAIN production readiness probe did not return ok.");
  }

  const bundleDecision = evaluateBundle(config);
  reasons.push(...bundleDecision.reasons);

  const sentinelDecision = evaluateSentinels({
    sentinels,
    policy: config.sentinelPolicy,
  });
  reasons.push(...sentinelDecision.reasons);

  const evidenceVerified =
    reasons.length === 0 &&
    productionHealthVerified &&
    productionReadyVerified &&
    bundleDecision.verified;

  return {
    evidence_state: evidenceVerified ? "verified" : "blocked",
    production_base_url: config.production?.baseUrl || null,
    production_health_verified: productionHealthVerified,
    production_ready_verified: productionReadyVerified,
    root_trust_bundle_verified: bundleDecision.verified,
    root_trust_bundle_id: config.rootTrustBundle?.bundleIdHex || null,
    root_trust_ceremony_id: config.rootTrustBundle?.ceremonyId || null,
    root_trust_issuer_did: config.rootTrustBundle?.issuerDid || null,
    verifier_commit: config.verification?.verifierCommit || null,
    verified_at: config.verification?.verifiedAt || null,
    source_basis: config.sourceBasis || [],
    reasons,
    non_blocking_observations: sentinelDecision.nonBlockingObservations,
  };
}

const exochainProductionTrustEvidence =
  evaluateExochainProductionTrustEvidence();

module.exports = {
  evaluateExochainProductionTrustEvidence,
  exochainProductionTrustConfig,
  exochainProductionTrustEvidence,
};
