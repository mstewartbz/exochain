function buildPublicRecordRequestResponse(recordRequest = {}) {
  return {
    id: recordRequest.id,
    provider_name: recordRequest.provider_name || null,
    provider_npi: recordRequest.provider_npi || null,
    provider_address: recordRequest.provider_address || null,
    status: recordRequest.status || null,
    letter_ready: Boolean(recordRequest.letter_pdf_path),
    sent_at: recordRequest.sent_at || null,
    pending_at: recordRequest.pending_at || null,
    received_at: recordRequest.received_at || null,
    created_at: recordRequest.created_at || null,
    updated_at: recordRequest.updated_at || null,
  };
}

module.exports = {
  buildPublicRecordRequestResponse,
};
