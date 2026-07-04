import { describe, expect, it } from "vitest";

const {
  buildPublicRecordRequestResponse,
} = require("../server/utils/record-request-response.js");

describe("record request response redaction", () => {
  it("returns bounded HIPAA request metadata without raw row internals", () => {
    const response = buildPublicRecordRequestResponse({
      id: 42,
      subscriber_id: 11,
      provider_name: "Mercy General",
      provider_npi: "1234567890",
      provider_address: "100 Health Way, Raleigh, NC",
      status: "sent",
      letter_pdf_path: "hipaa/42-request-letter.pdf",
      status_notes: "fax retried twice",
      sent_at: "2026-06-06T07:00:00.000Z",
      pending_at: null,
      received_at: null,
      created_at: "2026-06-06T06:59:55.000Z",
      updated_at: "2026-06-06T07:00:00.000Z",
    });

    expect(response).toEqual({
      id: 42,
      provider_name: "Mercy General",
      provider_npi: "1234567890",
      provider_address: "100 Health Way, Raleigh, NC",
      status: "sent",
      letter_ready: true,
      sent_at: "2026-06-06T07:00:00.000Z",
      pending_at: null,
      received_at: null,
      created_at: "2026-06-06T06:59:55.000Z",
      updated_at: "2026-06-06T07:00:00.000Z",
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("letter_pdf_path");
    expect(response).not.toHaveProperty("status_notes");
  });
});
