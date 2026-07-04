import { describe, expect, it } from "vitest";

const {
  buildInactiveConsentAuditMetadata,
  buildConsentGrantSuccessMessage,
  buildConsentRevocationSuccessMessage,
} = require("../server/utils/consent-audit-metadata.js");

describe("consent audit EXOCHAIN claim boundary", () => {
  it("keeps consent-grant audit metadata fail-closed", () => {
    const metadata = buildInactiveConsentAuditMetadata({
      event_type: "consent_granted",
      consent_id: 77,
      subscriber_did: "did:exo:subscriber:test",
      provider_did: "did:exo:provider:test",
      provider_name: "Dr. Example",
      purpose: "ongoing_medical_care",
      granted_at: "2026-06-03T19:30:00.000Z",
      expires_at: "2026-06-10T19:30:00.000Z",
    });

    expect(metadata).toMatchObject({
      event_type: "consent_granted",
      consent_id: 77,
      subscriber_did: "did:exo:subscriber:test",
      provider_did: "did:exo:provider:test",
      provider_name: "Dr. Example",
      purpose: "ongoing_medical_care",
      granted_at: "2026-06-03T19:30:00.000Z",
      expires_at: "2026-06-10T19:30:00.000Z",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.note).toBe(
      "Consent grant recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(metadata).not.toHaveProperty("exochain_anchored");
    expect(metadata).not.toHaveProperty("exochain_event");
  });

  it("keeps consent-revocation audit metadata fail-closed", () => {
    const metadata = buildInactiveConsentAuditMetadata({
      event_type: "consent_revoked",
      consent_id: 77,
      subscriber_did: "did:exo:subscriber:test",
      provider_did: "did:exo:provider:test",
      provider_name: "Dr. Example",
      revoked_at: "2026-06-03T20:00:00.000Z",
    });

    expect(metadata).toMatchObject({
      event_type: "consent_revoked",
      consent_id: 77,
      subscriber_did: "did:exo:subscriber:test",
      provider_did: "did:exo:provider:test",
      provider_name: "Dr. Example",
      revoked_at: "2026-06-03T20:00:00.000Z",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.note).toBe(
      "Consent revocation recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(metadata).not.toHaveProperty("exochain_anchored");
    expect(metadata).not.toHaveProperty("exochain_event");
  });

  it("uses fail-closed consent response copy", () => {
    expect(buildConsentGrantSuccessMessage()).toBe(
      "Consent granted. Local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(buildConsentRevocationSuccessMessage()).toBe(
      "Consent revoked. Local audit receipt recorded while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
  });
});
