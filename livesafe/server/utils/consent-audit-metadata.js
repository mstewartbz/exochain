"use strict";

const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");

const CONSENT_NOTES = {
  consent_granted:
    "Consent grant recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  consent_revoked:
    "Consent revocation recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
};

function buildInactiveConsentAuditMetadata(metadata = {}) {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();
  const eventType = metadata.event_type;

  return {
    ...metadata,
    exochain_anchor_state: "not_called",
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    note:
      CONSENT_NOTES[eventType] ||
      "Consent event recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  };
}

function buildConsentGrantSuccessMessage() {
  return "Consent granted. Local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.";
}

function buildConsentRevocationSuccessMessage() {
  return "Consent revoked. Local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.";
}

module.exports = {
  buildInactiveConsentAuditMetadata,
  buildConsentGrantSuccessMessage,
  buildConsentRevocationSuccessMessage,
};
