"use strict";

const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");

const DELETION_NOTES = {
  subscriber_account:
    "Subscriber self-deleted account; local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  medical_record_copy:
    "Subscriber deleted their copy; local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
};

function buildInactiveDeletionAuditMetadata(metadata = {}) {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();
  const deletionKind = metadata.deletion_kind;

  return {
    ...metadata,
    exochain_anchor_state: "not_called",
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    note:
      DELETION_NOTES[deletionKind] ||
      "Local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  };
}

module.exports = {
  buildInactiveDeletionAuditMetadata,
};
