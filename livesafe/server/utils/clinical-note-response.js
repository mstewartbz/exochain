function buildPublicClinicalNoteResponse(note = {}) {
  const derivedProviderDisplayName = [note.provider_first_name, note.provider_last_name]
    .filter(Boolean)
    .join(" ")
    .trim();

  return {
    id: note.id ?? null,
    note_text: note.note_text ?? null,
    note_type: note.note_type ?? null,
    status: note.status ?? null,
    provider_display_name:
      (note.provider_display_name ?? derivedProviderDisplayName) || null,
    provider_first_name: note.provider_first_name ?? null,
    provider_last_name: note.provider_last_name ?? null,
    subscriber_first_name: note.subscriber_first_name ?? null,
    subscriber_last_name: note.subscriber_last_name ?? null,
    approved_at: note.approved_at ?? null,
    rejected_at: note.rejected_at ?? null,
    created_at: note.created_at ?? null,
    updated_at: note.updated_at ?? null,
  };
}

function buildPublicClinicalNoteListResponse(notes = []) {
  return notes.map(buildPublicClinicalNoteResponse);
}

function buildPublicClinicalNoteCreateAcknowledgement({
  note = {},
  message = null,
} = {}) {
  return {
    success: true,
    note: buildPublicClinicalNoteResponse(note),
    message,
  };
}

function buildPublicClinicalNoteListEnvelope(notes = []) {
  const responseNotes = buildPublicClinicalNoteListResponse(notes);

  return {
    notes: responseNotes,
    pending_count: responseNotes.filter((note) => note.status === "pending_approval").length,
    approved_count: responseNotes.filter((note) => note.status === "approved").length,
    rejected_count: responseNotes.filter((note) => note.status === "rejected").length,
  };
}

function buildPublicClinicalNoteMutationAcknowledgement({
  note = {},
  message = null,
} = {}) {
  return {
    success: true,
    note: buildPublicClinicalNoteResponse(note),
    message,
  };
}

module.exports = {
  buildPublicClinicalNoteCreateAcknowledgement,
  buildPublicClinicalNoteListEnvelope,
  buildPublicClinicalNoteListResponse,
  buildPublicClinicalNoteMutationAcknowledgement,
  buildPublicClinicalNoteResponse,
};
