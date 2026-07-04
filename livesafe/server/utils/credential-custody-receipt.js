"use strict";

const { runtimeExochainAdapter } = require("./livesafe-exochain-adapter");

const RECEIPT_NOTES = {
  advance_directive:
    "Advance directive stored as an encrypted local custody record while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  power_of_attorney:
    "Power of attorney stored as an encrypted local custody record while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
};

const SUCCESS_MESSAGES = {
  advance_directive:
    "Advance directive uploaded to encrypted local custody successfully",
  power_of_attorney:
    "Power of Attorney stored in encrypted local custody successfully",
};

function buildInactiveCredentialCustodyReceipt(receipt = {}) {
  const runtimeStatus = runtimeExochainAdapter.getRuntimeStatus();
  const assetType = receipt.asset_type;

  return {
    ...receipt,
    receipt_type: "LOCAL_ENCRYPTED_CUSTODY",
    custody_state: "local_only",
    exochain_anchor_state: "not_called",
    runtime_adapter_state: runtimeStatus.adapter_state,
    public_claims_allowed: runtimeStatus.public_claims_allowed,
    note:
      RECEIPT_NOTES[assetType] ||
      "Credential stored as an encrypted local custody record while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
  };
}

function buildCredentialCustodySuccessMessage({ asset_type: assetType } = {}) {
  return (
    SUCCESS_MESSAGES[assetType] ||
    "Credential stored in encrypted local custody successfully"
  );
}

module.exports = {
  buildInactiveCredentialCustodyReceipt,
  buildCredentialCustodySuccessMessage,
};
