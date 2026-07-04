import { describe, expect, it } from "vitest";

const {
  buildPublicScanCreateResponse,
  buildScanPostActionFailureResponse,
} = require("../server/routes/scan.js");

describe("scan create response redaction", () => {
  it("redacts raw scan-row internals and trustee recipient identifiers from successful scan responses", () => {
    const response = buildPublicScanCreateResponse({
      scan: {
        id: 42,
        subscriber_id: 7,
        responder_id: 11,
        card_id: 15,
        access_token: "secret-scan-token",
        access_expires_at: "2026-06-05T12:00:00.000Z",
        location_lat: 38.8977,
        location_lng: -77.0365,
        location: "1600 Pennsylvania Ave NW",
        scan_type_text: "emergency",
        scanned_at: "2026-06-05T11:25:00.000Z",
        created_at: "2026-06-05T11:25:00.000Z",
        updated_at: "2026-06-05T11:25:00.000Z",
      },
      paceAlertsSent: 2,
      paceAlertDispatchedAt: "2026-06-05T11:25:04.000Z",
      paceAlertsDeduplicated: 1,
      paceAlertDedupWindowMinutes: 5,
      paceAlertNotifications: [
        {
          id: 91,
          sent_at: "2026-06-05T11:25:03.000Z",
          recipient_did: "did:exo:trustee:one",
          channel: "sms",
        },
      ],
    });

    expect(response).toEqual({
      id: 42,
      scanned_at: "2026-06-05T11:25:00.000Z",
      scan_type: "emergency",
      access_expires_at: "2026-06-05T12:00:00.000Z",
      pace_alerts_sent: 2,
      pace_alert_dispatched_at: "2026-06-05T11:25:04.000Z",
      pace_alert_notifications: [
        {
          id: 91,
          sent_at: "2026-06-05T11:25:03.000Z",
          channel: "sms",
        },
      ],
      pace_alerts_deduplicated: 1,
      pace_alert_dedup_window_minutes: 5,
    });

    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("responder_id");
    expect(response).not.toHaveProperty("card_id");
    expect(response).not.toHaveProperty("access_token");
    expect(response).not.toHaveProperty("location");
    expect(response).not.toHaveProperty("location_lat");
    expect(response).not.toHaveProperty("location_lng");
    expect(JSON.stringify(response)).not.toContain("did:exo:trustee:one");
  });

  it("keeps post-scan P.A.C.E. alert delivery failures bounded on the same redacted response shape", () => {
    const response = buildScanPostActionFailureResponse({
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
    expect(response).not.toHaveProperty("access_token");
    expect(response).not.toHaveProperty("location");
    expect(JSON.stringify(response)).not.toContain("Twilio auth token invalid");
    expect(JSON.stringify(response)).not.toContain("AC123");
  });
});
