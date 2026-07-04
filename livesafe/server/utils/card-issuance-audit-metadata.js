"use strict";

const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");

function buildInactiveCardIssuanceAuditMetadata(metadata = {}) {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();

  return {
    ...metadata,
    exochain_anchor_state: "not_called",
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    note:
      "Emergency card issuance recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  };
}

module.exports = {
  buildInactiveCardIssuanceAuditMetadata,
};
