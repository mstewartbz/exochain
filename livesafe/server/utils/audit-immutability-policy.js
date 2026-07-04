"use strict";

const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");

const LOCAL_AUDIT_IMMUTABILITY_POLICY = "LIVESAFE_LOCAL_AUDIT_IMMUTABILITY_v1";
const LOCAL_AUDIT_IMMUTABILITY_NOTE =
  "Local audit receipts remain append-only through this LiveSafe surface while EXOCHAIN anchoring stays inactive until a verified adapter path is invoked.";

function buildAuditImmutabilityError() {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();

  return {
    error: "Audit trail is immutable",
    message:
      "Audit records cannot be modified or deleted through this LiveSafe surface. This route enforces local audit immutability while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    policy: LOCAL_AUDIT_IMMUTABILITY_POLICY,
    audit_storage_classification: "local_audit_receipt",
    exochain_anchor_state: "not_called",
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    note: LOCAL_AUDIT_IMMUTABILITY_NOTE,
  };
}

module.exports = {
  buildAuditImmutabilityError,
  LOCAL_AUDIT_IMMUTABILITY_NOTE,
  LOCAL_AUDIT_IMMUTABILITY_POLICY,
};
