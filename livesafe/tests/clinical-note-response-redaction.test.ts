import { describe, expect, it } from "vitest";

const {
  buildPublicClinicalNoteCreateAcknowledgement,
  buildPublicClinicalNoteListResponse,
  buildPublicClinicalNoteListEnvelope,
  buildPublicClinicalNoteMutationAcknowledgement,
  buildPublicClinicalNoteResponse,
} = require("../server/utils/clinical-note-response.js");

describe("clinical note response redaction", () => {
  it("returns bounded clinical-note metadata without provider or subscriber internals", () => {
    const response = buildPublicClinicalNoteResponse({
      id: 17,
      subscriber_id: 11,
      provider_id: 5,
      note_text: "Observed follow-up improvement after discharge.",
      note_type: "progress_note",
      status: "pending_approval",
      provider_display_name: "Ada Lovelace Clinic",
      provider_first_name: "Ada",
      provider_last_name: "Lovelace",
      provider_email: "ada@example.com",
      provider_did: "did:exo:provider:ada",
      subscriber_first_name: "Grace",
      subscriber_last_name: "Hopper",
      subscriber_did: "did:exo:subscriber:grace",
      reject_reason: "Too much detail",
      approved_at: null,
      rejected_at: null,
      created_at: "2026-06-06T12:30:00.000Z",
      updated_at: "2026-06-06T12:31:00.000Z",
    });

    expect(response).toEqual({
      id: 17,
      note_text: "Observed follow-up improvement after discharge.",
      note_type: "progress_note",
      status: "pending_approval",
      provider_display_name: "Ada Lovelace Clinic",
      provider_first_name: "Ada",
      provider_last_name: "Lovelace",
      subscriber_first_name: "Grace",
      subscriber_last_name: "Hopper",
      approved_at: null,
      rejected_at: null,
      created_at: "2026-06-06T12:30:00.000Z",
      updated_at: "2026-06-06T12:31:00.000Z",
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("provider_id");
    expect(response).not.toHaveProperty("provider_email");
    expect(response).not.toHaveProperty("provider_did");
    expect(response).not.toHaveProperty("subscriber_did");
    expect(response).not.toHaveProperty("reject_reason");
  });

  it("returns bounded clinical-note list entries", () => {
    const notes = buildPublicClinicalNoteListResponse([
      {
        id: 18,
        note_text: "Cardiology referral recommended.",
        note_type: "referral_note",
        status: "approved",
        provider_first_name: "Ada",
        provider_last_name: "Lovelace",
        subscriber_first_name: "Grace",
        subscriber_last_name: "Hopper",
        provider_email: "ada@example.com",
        subscriber_did: "did:exo:subscriber:grace",
        created_at: "2026-06-06T12:32:00.000Z",
        updated_at: "2026-06-06T12:33:00.000Z",
      },
    ]);

    expect(notes).toEqual([
      {
        id: 18,
        note_text: "Cardiology referral recommended.",
        note_type: "referral_note",
        status: "approved",
        provider_display_name: "Ada Lovelace",
        provider_first_name: "Ada",
        provider_last_name: "Lovelace",
        subscriber_first_name: "Grace",
        subscriber_last_name: "Hopper",
        approved_at: null,
        rejected_at: null,
        created_at: "2026-06-06T12:32:00.000Z",
        updated_at: "2026-06-06T12:33:00.000Z",
      },
    ]);
    expect(notes[0]).not.toHaveProperty("provider_email");
    expect(notes[0]).not.toHaveProperty("subscriber_did");
  });

  it("returns a bounded create acknowledgement without raw note rows", () => {
    expect(
      buildPublicClinicalNoteCreateAcknowledgement({
        note: {
          id: 19,
          subscriber_id: 11,
          provider_id: 5,
          note_text: "Observed follow-up improvement after discharge.",
          note_type: "progress_note",
          status: "pending_approval",
          provider_email: "ada@example.com",
          provider_did: "did:exo:provider:ada",
        },
        message: "Clinical note submitted and awaiting subscriber approval.",
      }),
    ).toEqual({
      success: true,
      note: {
        id: 19,
        note_text: "Observed follow-up improvement after discharge.",
        note_type: "progress_note",
        status: "pending_approval",
        provider_display_name: null,
        provider_first_name: null,
        provider_last_name: null,
        subscriber_first_name: null,
        subscriber_last_name: null,
        approved_at: null,
        rejected_at: null,
        created_at: null,
        updated_at: null,
      },
      message: "Clinical note submitted and awaiting subscriber approval.",
    });
  });

  it("returns a bounded list envelope with derived counts", () => {
    expect(
      buildPublicClinicalNoteListEnvelope([
        { id: 1, status: "pending_approval" },
        { id: 2, status: "approved" },
        { id: 3, status: "rejected" },
      ]),
    ).toEqual({
      notes: [
        expect.objectContaining({ id: 1, status: "pending_approval" }),
        expect.objectContaining({ id: 2, status: "approved" }),
        expect.objectContaining({ id: 3, status: "rejected" }),
      ],
      pending_count: 1,
      approved_count: 1,
      rejected_count: 1,
    });
  });

  it("returns bounded mutation acknowledgements without reject reasons", () => {
    expect(
      buildPublicClinicalNoteMutationAcknowledgement({
        note: {
          id: 20,
          note_text: "Referral recommended.",
          note_type: "referral_note",
          status: "rejected",
          reject_reason: "too much detail",
        },
        message: "Clinical note rejected.",
      }),
    ).toEqual({
      success: true,
      note: {
        id: 20,
        note_text: "Referral recommended.",
        note_type: "referral_note",
        status: "rejected",
        provider_display_name: null,
        provider_first_name: null,
        provider_last_name: null,
        subscriber_first_name: null,
        subscriber_last_name: null,
        approved_at: null,
        rejected_at: null,
        created_at: null,
        updated_at: null,
      },
      message: "Clinical note rejected.",
    });
  });
});
