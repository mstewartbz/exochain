import { describe, expect, it } from "vitest";

const {
  buildInactiveCredentialCustodyReceipt,
  buildCredentialCustodySuccessMessage,
} = require("../server/utils/credential-custody-receipt.js");

describe("credential custody EXOCHAIN claim boundary", () => {
  it("keeps advance-directive custody receipts fail-closed", () => {
    const receipt = buildInactiveCredentialCustodyReceipt({
      receipt_id: "custody:receipt:advance-directive:test",
      subscriber_did: "did:exo:subscriber:test",
      asset_type: "advance_directive",
      asset_hash: "a".repeat(64),
      timestamp: "2026-06-03T18:30:00.000Z",
    });

    expect(receipt).toMatchObject({
      receipt_id: "custody:receipt:advance-directive:test",
      receipt_type: "LOCAL_ENCRYPTED_CUSTODY",
      subscriber_did: "did:exo:subscriber:test",
      asset_type: "advance_directive",
      asset_hash: "a".repeat(64),
      timestamp: "2026-06-03T18:30:00.000Z",
      custody_state: "local_only",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(receipt.note).toBe(
      "Advance directive stored as an encrypted local custody record while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(receipt).not.toHaveProperty("chain");
    expect(receipt).not.toHaveProperty("type");
  });

  it("keeps power-of-attorney custody receipts fail-closed", () => {
    const receipt = buildInactiveCredentialCustodyReceipt({
      receipt_id: "custody:receipt:poa:test",
      subscriber_did: "did:exo:subscriber:test",
      asset_type: "power_of_attorney",
      asset_hash: "b".repeat(64),
      timestamp: "2026-06-03T18:31:00.000Z",
      pace_trustee_did: "did:exo:trustee:test",
    });

    expect(receipt).toMatchObject({
      receipt_id: "custody:receipt:poa:test",
      receipt_type: "LOCAL_ENCRYPTED_CUSTODY",
      subscriber_did: "did:exo:subscriber:test",
      asset_type: "power_of_attorney",
      asset_hash: "b".repeat(64),
      timestamp: "2026-06-03T18:31:00.000Z",
      pace_trustee_did: "did:exo:trustee:test",
      custody_state: "local_only",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(receipt.note).toBe(
      "Power of attorney stored as an encrypted local custody record while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(receipt).not.toHaveProperty("chain");
    expect(receipt).not.toHaveProperty("type");
  });

  it("keeps credential-upload success copy fail-closed", () => {
    expect(
      buildCredentialCustodySuccessMessage({ asset_type: "advance_directive" }),
    ).toBe("Advance directive uploaded to encrypted local custody successfully");
    expect(
      buildCredentialCustodySuccessMessage({ asset_type: "power_of_attorney" }),
    ).toBe("Power of Attorney stored in encrypted local custody successfully");
  });
});
