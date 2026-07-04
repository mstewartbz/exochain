import { describe, expect, it } from "vitest";

const scanRoutes = require("../server/routes/scan.js");

describe("scan route EXOCHAIN payload boundary", () => {
  it("builds a metadata-only EXOCHAIN scan payload without explicit location", () => {
    const payload = scanRoutes.buildExochainScanAnchorInput({
      scan: {
        id: 42,
        access_expires_at: "2026-06-03T19:00:00.000Z",
      },
      subscriberDid: "did:exo:subscriber:test",
      responderDid: "did:exo:responder:test",
      scanTimestamp: "2026-06-03T15:40:28.061Z",
      location: "123 Main St, Raleigh, NC",
      auditReceiptHash:
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    });

    expect(payload).toEqual({
      scanId: 42,
      subscriberDid: "did:exo:subscriber:test",
      responderDid: "did:exo:responder:test",
      scannedAtMs: Date.parse("2026-06-03T15:40:28.061Z"),
      consentExpiresAtMs: Date.parse("2026-06-03T19:00:00.000Z"),
      auditReceiptHash:
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    });
    expect(Object.prototype.hasOwnProperty.call(payload, "location")).toBe(false);
  });

  it("keeps local scan audit metadata public-claims fail-closed with a verified runtime adapter", () => {
    const metadata = scanRoutes.buildInactiveScanAuditMetadata({
      scan_id: 42,
      scan_timestamp: "2026-06-03T18:55:39.114Z",
      responder_agency: "Wake EMS",
      responder_did: "did:exo:responder:test",
      location: "123 Main St, Raleigh, NC",
      subscriber_did: "did:exo:subscriber:test",
      scan_type: "emergency",
    });

    expect(metadata).toMatchObject({
      scan_id: 42,
      scan_timestamp: "2026-06-03T18:55:39.114Z",
      responder_agency: "Wake EMS",
      responder_did: "did:exo:responder:test",
      location: "123 Main St, Raleigh, NC",
      subscriber_did: "did:exo:subscriber:test",
      scan_type: "emergency",
      exochain_anchor_state: "not_called",
      runtime_adapter_state: "verified",
      public_claims_allowed: false,
    });
    expect(metadata.note).toBe(
      "Emergency card scan recorded in a local audit receipt while EXOCHAIN anchoring remains inactive until a verified adapter path is invoked.",
    );
    expect(metadata).not.toHaveProperty("exochain_event");
    expect(metadata).not.toHaveProperty("exochain_anchored");
  });

  it("redacts post-scan P.A.C.E. alert delivery failures from the public scan response", () => {
    const response = scanRoutes.buildScanPostActionFailureResponse({
      scan: {
        id: 42,
        subscriber_id: 7,
        responder_id: 11,
        access_token: "scan-token",
        access_expires_at: "2026-06-05T12:00:00.000Z",
        location: "1600 Pennsylvania Ave NW",
        scan_type_text: "emergency",
        scanned_at: "2026-06-05T11:25:00.000Z",
      },
      error: new Error("Twilio auth token invalid for account AC123"),
    });

    expect(response).toEqual({
      id: 42,
      scanned_at: "2026-06-05T11:25:00.000Z",
      scan_type: "emergency",
      access_expires_at: "2026-06-05T12:00:00.000Z",
      pace_alerts_sent: 0,
      pace_alert_delivery: {
        status: "failed",
        reason: "notification_delivery_failed",
      },
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("responder_id");
    expect(response).not.toHaveProperty("access_token");
    expect(response).not.toHaveProperty("location");
    expect(response).not.toHaveProperty("pace_alert_error");
    expect(JSON.stringify(response)).not.toContain("Twilio auth token invalid");
    expect(JSON.stringify(response)).not.toContain("AC123");
  });
});
