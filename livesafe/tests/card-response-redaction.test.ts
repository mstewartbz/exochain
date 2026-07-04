const {
  buildPublicCardIssueResponse,
  buildPublicCardNfcResponse,
  buildPublicCardStatusResponse,
} = require("../server/utils/card-response.js");

describe("card response redaction", () => {
  const cardRow = {
    id: 44,
    subscriber_id: 91,
    qr_data: "data:image/png;base64,secret",
    nfc_payload: '{"did":"did:exo:subscriber:test","emergency_token":"secret"}',
    emergency_consent_token: "secret-token-value",
    status: "active",
    issued_at: "2026-06-06T19:40:00.000Z",
    expires_at: "2027-06-06T19:40:00.000Z",
  };

  it("builds a bounded card issue response without QR or NFC secrets", () => {
    expect(
      buildPublicCardIssueResponse({
        card: cardRow,
        alreadyIssued: true,
        message: "Card already issued",
        qrImageUrl: "/api/card/did%3Aexo%3Asubscriber%3Atest/qr",
      }),
    ).toEqual({
      card: {
        status: "active",
        issued_at: "2026-06-06T19:40:00.000Z",
        expires_at: "2027-06-06T19:40:00.000Z",
        qr_image_url: "/api/card/did%3Aexo%3Asubscriber%3Atest/qr",
        qr_pointer_state: "pointer-only",
        nfc_pointer_state: "pointer-only",
      },
      already_issued: true,
      message: "Card already issued",
    });
  });

  it("builds a bounded card status response without raw card rows", () => {
    expect(
      buildPublicCardStatusResponse({
        card: cardRow,
        qrImageUrl: "/api/card/did%3Aexo%3Asubscriber%3Atest/qr",
        compositeScore: 42.337,
        identityCoreScore: 18.125,
        canIssue: true,
        minimumScore: 10,
        paceComplete: true,
        acceptedTrustees: 4,
        requiredTrustees: 4,
      }),
    ).toEqual({
      card: {
        status: "active",
        issued_at: "2026-06-06T19:40:00.000Z",
        expires_at: "2027-06-06T19:40:00.000Z",
        qr_image_url: "/api/card/did%3Aexo%3Asubscriber%3Atest/qr",
        qr_pointer_state: "pointer-only",
        nfc_pointer_state: "pointer-only",
      },
      composite_score: 42.34,
      identity_core_score: 18.13,
      can_issue: true,
      minimum_score: 10,
      pace_complete: true,
      accepted_trustees: 4,
      required_trustees: 4,
    });
  });

  it("builds a metadata-only NFC payload without emergency token or card identifiers", () => {
    expect(
      buildPublicCardNfcResponse({
        subscriberDid: "did:exo:subscriber:test",
        hasActiveCard: true,
      }),
    ).toEqual({
      nfc_payload: {
        did: "did:exo:subscriber:test",
        type: "emergency_access",
      },
      matches_qr: true,
      pointer_state: "metadata-only",
    });
  });
});
