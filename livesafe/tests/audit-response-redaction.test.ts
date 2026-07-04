import { describe, expect, it } from "vitest";

const {
  buildAuditEventResponse,
  buildAuditTrailResponse,
} = require("../server/utils/audit-response.js");

describe("audit response redaction", () => {
  it("returns a bounded record-deletion audit event without raw audit internals", () => {
    const response = buildAuditEventResponse({
      id: 42,
      subject_did: "did:exo:subscriber:123",
      actor_did: "did:exo:subscriber:123",
      event_type: "record_deleted",
      scope: "medical_records",
      details: JSON.stringify({
        record_title: "Allergy Card",
        record_type: "credential",
        note: "Subscriber deleted their copy; local audit receipt recorded.",
        subscriber_email: "person@example.com",
        raw_record_blob: "<xml>secret</xml>",
      }),
      receipt_hash: "abcd1234",
      previous_hash: "prev-hash",
      exochain_receipt: "exo-anchor-ref",
      created_at: "2026-06-06T05:00:00.000Z",
    });

    expect(response).toEqual({
      id: 42,
      actor_did: "did:exo:subscriber:123",
      event_type: "record_deleted",
      scope: "medical_records",
      details: {
        record_title: "Allergy Card",
        record_type: "credential",
        note: "Subscriber deleted their copy; local audit receipt recorded.",
      },
      receipt_hash: "abcd1234",
      created_at: "2026-06-06T05:00:00.000Z",
    });
    expect(response).not.toHaveProperty("subject_did");
    expect(response).not.toHaveProperty("previous_hash");
    expect(response).not.toHaveProperty("exochain_receipt");
  });

  it("drops raw detail blobs for non-allowlisted audit events", () => {
    const response = buildAuditEventResponse({
      id: 7,
      actor_did: "did:exo:provider:abc",
      event_type: "consent_granted",
      scope: "provider_access",
      details: {
        provider_email: "doctor@example.com",
        access_token: "raw-secret",
      },
      receipt_hash: "receipt-7",
      created_at: "2026-06-06T05:01:00.000Z",
    });

    expect(response).toEqual({
      id: 7,
      actor_did: "did:exo:provider:abc",
      event_type: "consent_granted",
      scope: "provider_access",
      details: null,
      receipt_hash: "receipt-7",
      created_at: "2026-06-06T05:01:00.000Z",
    });
  });

  it("maps audit trails through the bounded event helper", () => {
    expect(
      buildAuditTrailResponse([
        {
          id: 1,
          event_type: "record_deleted",
          details: { record_title: "Packet" },
          created_at: "2026-06-06T05:02:00.000Z",
        },
      ]),
    ).toEqual([
      {
        id: 1,
        actor_did: null,
        event_type: "record_deleted",
        scope: null,
        details: { record_title: "Packet" },
        receipt_hash: null,
        created_at: "2026-06-06T05:02:00.000Z",
      },
    ]);
  });
});
