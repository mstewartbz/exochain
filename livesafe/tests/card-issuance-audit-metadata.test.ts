import { describe, expect, it } from "vitest";

const {
  buildInactiveCardIssuanceAuditMetadata,
} = require("../server/utils/card-issuance-audit-metadata.js");

describe("card issuance EXOCHAIN claim boundary", () => {
  it("keeps card-issuance audit metadata fail-closed", () => {
    const metadata = buildInactiveCardIssuanceAuditMetadata({
      card_id: 44,
      subscriber_did: "did:exo:subscriber:test",
      issued_at: "2026-06-03T18:45:00.000Z",
      status: "active",
      emergency_consent_token_ref: "12345678...",
    });

    expect(metadata).toMatchObject({
      card_id: 44,
      subscriber_did: "did:exo:subscriber:test",
      issued_at: "2026-06-03T18:45:00.000Z",
      status: "active",
      emergency_consent_token_ref: "12345678...",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.note).toBe(
      "Emergency card issuance recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(metadata).not.toHaveProperty("exochain_event");
    expect(metadata).not.toHaveProperty("exochain_anchored");
  });
});
