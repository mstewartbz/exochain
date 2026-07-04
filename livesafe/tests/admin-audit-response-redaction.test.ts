const {
  buildAdminAuditTrailResponse,
} = require("../server/utils/admin-audit-response.js");

describe("admin audit response redaction", () => {
  const auditRow = {
    id: 44,
    subject_did: "did:exo:subscriber:private",
    actor_did: "did:exo:provider:visible",
    event_type: "record_deleted",
    scope: "medical_records",
    details: JSON.stringify({
      record_title: "Medication Summary",
      record_type: "clinical_note",
      note: "Subscriber removed this copy from the vault.",
      subscriber_email: "private@example.com",
      raw_blob: "<xml>secret</xml>",
    }),
    receipt_hash: "receipt-44",
    previous_hash: "prev-44",
    exochain_receipt: "exo-anchor-44",
    subscriber_email: "private@example.com",
    created_at: "2026-06-07T02:58:00.000Z",
  };

  it("builds bounded admin audit collections without raw receipt rows", () => {
    expect(
      buildAdminAuditTrailResponse([auditRow], {
        total: 1,
        page: 2,
        limit: 10,
      }),
    ).toEqual({
      records: [
        {
          id: 44,
          actor_did: "did:exo:provider:visible",
          event_type: "record_deleted",
          scope: "medical_records",
          details: {
            record_title: "Medication Summary",
            record_type: "clinical_note",
            note: "Subscriber removed this copy from the vault.",
          },
          receipt_hash: "receipt-44",
          created_at: "2026-06-07T02:58:00.000Z",
        },
      ],
      total: 1,
      page: 2,
      limit: 10,
    });
  });
});
