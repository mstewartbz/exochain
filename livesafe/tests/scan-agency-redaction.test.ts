import { describe, expect, it } from "vitest";

const {
  buildAgencyScanSummary,
  buildAgencyResponderSummary,
  buildScanFollowupMutationResponse,
} = require("../server/routes/scan.js");

describe("scan agency response redaction", () => {
  it("redacts raw access, location, and subscriber email fields from agency scan summaries", () => {
    const response = buildAgencyScanSummary({
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
      responder_email: "medic@example.com",
      responder_role: "agency_admin",
      subscriber_did: "did:exo:subscriber:123",
      subscriber_email: "subscriber@example.com",
      flagged_for_followup: true,
      followup_notes: "Call ER desk back.",
    });

    expect(response).toEqual({
      scan_id: 42,
      scanned_at: "2026-06-05T11:25:00.000Z",
      scan_type: "emergency",
      access_expires_at: "2026-06-05T12:00:00.000Z",
      subscriber_did: "did:exo:subscriber:123",
      responder_email: "medic@example.com",
      responder_role: "agency_admin",
      flagged_for_followup: true,
      followup_notes: "Call ER desk back.",
    });

    expect(response).not.toHaveProperty("subscriber_email");
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("responder_id");
    expect(response).not.toHaveProperty("card_id");
    expect(response).not.toHaveProperty("access_token");
    expect(response).not.toHaveProperty("location");
    expect(response).not.toHaveProperty("location_lat");
    expect(response).not.toHaveProperty("location_lng");
  });

  it("redacts responder DID fields from agency responder summaries", () => {
    const response = buildAgencyResponderSummary({
      id: 11,
      did: "did:exo:responder:abc123",
      email: "medic@example.com",
      role: "agency_admin",
      certification: "NREMT-P",
    });

    expect(response).toEqual({
      responder_id: 11,
      email: "medic@example.com",
      role: "agency_admin",
      certification: "NREMT-P",
    });

    expect(response).not.toHaveProperty("did");
    expect(response).not.toHaveProperty("id");
  });

  it("keeps scan follow-up acknowledgements bounded without raw scan ids or freeform notes", () => {
    const response = buildScanFollowupMutationResponse({
      scan: {
        id: 42,
        flagged_for_followup: true,
        followup_notes: "Call ER desk back about cardiac escalation.",
      },
      alreadyFlagged: true,
    });

    expect(response).toEqual({
      flagged_for_followup: true,
      message: "Scan already flagged for follow-up",
      already_flagged: true,
      followup_notes_present: true,
    });

    expect(response).not.toHaveProperty("id");
    expect(response).not.toHaveProperty("followup_notes");
    expect(JSON.stringify(response)).not.toContain("cardiac escalation");
  });
});
