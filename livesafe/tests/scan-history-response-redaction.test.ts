import { describe, expect, it } from "vitest";

const {
  buildSubscriberScanHistoryEntry,
  buildSubscriberScanDetailResponse,
} = require("../server/routes/scan.js");

describe("subscriber scan history response redaction", () => {
  const scanRow = {
    id: 42,
    subscriber_id: 7,
    responder_id: 11,
    card_id: 15,
    access_token: "secret-scan-token",
    access_expires_at: "2026-06-06T06:10:00.000Z",
    location_lat: 38.8977,
    location_lng: -77.0365,
    location: "1600 Pennsylvania Ave NW",
    scan_type_text: "emergency",
    scanned_at: "2026-06-06T05:55:00.000Z",
    responder_email: "medic@example.com",
    responder_did_val: "did:exo:responder:private",
    responder_role: "responder",
    agency_name: "Wake EMS",
    flagged_for_followup: true,
    followup_notes: "Call ER desk back.",
  };

  it("builds a bounded scan-history entry without responder identity or raw location fields", () => {
    expect(buildSubscriberScanHistoryEntry(scanRow)).toEqual({
      id: 42,
      scanned_at: "2026-06-06T05:55:00.000Z",
      scan_type: "emergency",
      access_expires_at: "2026-06-06T06:10:00.000Z",
      responder_role: "responder",
      agency_name: "Wake EMS",
      location_recorded: true,
    });
  });

  it("builds a bounded scan-detail response without responder identity, raw location, or scan-row internals", () => {
    expect(buildSubscriberScanDetailResponse(scanRow)).toEqual({
      id: 42,
      scanned_at: "2026-06-06T05:55:00.000Z",
      scan_type: "emergency",
      access_expires_at: "2026-06-06T06:10:00.000Z",
      responder_role: "responder",
      agency_name: "Wake EMS",
      location_recorded: true,
      flagged_for_followup: true,
    });
  });

  it("does not leak responder identity, raw location, token, or follow-up note fields", () => {
    const historyEntry = buildSubscriberScanHistoryEntry(scanRow);
    const detailEntry = buildSubscriberScanDetailResponse(scanRow);

    expect(historyEntry).not.toHaveProperty("subscriber_id");
    expect(historyEntry).not.toHaveProperty("responder_id");
    expect(historyEntry).not.toHaveProperty("card_id");
    expect(historyEntry).not.toHaveProperty("access_token");
    expect(historyEntry).not.toHaveProperty("location");
    expect(historyEntry).not.toHaveProperty("location_lat");
    expect(historyEntry).not.toHaveProperty("location_lng");
    expect(historyEntry).not.toHaveProperty("responder_email");
    expect(historyEntry).not.toHaveProperty("responder_did_val");

    expect(detailEntry).not.toHaveProperty("subscriber_id");
    expect(detailEntry).not.toHaveProperty("responder_id");
    expect(detailEntry).not.toHaveProperty("card_id");
    expect(detailEntry).not.toHaveProperty("access_token");
    expect(detailEntry).not.toHaveProperty("location");
    expect(detailEntry).not.toHaveProperty("location_lat");
    expect(detailEntry).not.toHaveProperty("location_lng");
    expect(detailEntry).not.toHaveProperty("followup_notes");
    expect(detailEntry).not.toHaveProperty("responder_email");
    expect(detailEntry).not.toHaveProperty("responder_did_val");
    expect(JSON.stringify(detailEntry)).not.toContain("medic@example.com");
    expect(JSON.stringify(detailEntry)).not.toContain("did:exo:responder:private");
    expect(JSON.stringify(detailEntry)).not.toContain("1600 Pennsylvania Ave NW");
    expect(JSON.stringify(detailEntry)).not.toContain("Call ER desk back.");
  });
});
