function buildPublicMedicalRecordResponse(record = {}) {
  return {
    id: record.id,
    title: record.title ?? null,
    record_type: record.record_type ?? null,
    category: record.category ?? null,
    file_format: record.file_format ?? null,
    file_size: record.file_size ?? null,
    extracted_data: record.extracted_data ?? null,
    annotation: record.annotation ?? null,
    encrypted: record.encrypted === true,
    visibility: record.visibility ?? null,
    visibility_providers: record.visibility_providers ?? null,
    version: record.version ?? 1,
    version_number: record.version ?? 1,
    download_available: Boolean(record.file_path),
    created_at: record.created_at ?? null,
    updated_at: record.updated_at ?? null,
  };
}

function buildPublicMedicalRecordListResponse(records = []) {
  return records.map(buildPublicMedicalRecordResponse);
}

function buildPublicMedicalRecordVersionListResponse(records = []) {
  return records.map(buildPublicMedicalRecordResponse);
}

function buildPublicMedicalRecordVersionEnvelope(records = []) {
  const versions = buildPublicMedicalRecordVersionListResponse(records);

  return {
    versions,
    total_versions: versions.length,
  };
}

function buildPublicMedicalRecordEncryptionStatusResponse(record = {}) {
  const encrypted = record.encrypted === true;

  return {
    record_id: record.id ?? null,
    title: record.title ?? null,
    encrypted,
    algorithm: encrypted ? 'AES-256-GCM' : null,
    key_type: encrypted ? 'subscriber-derived-PBKDF2' : null,
    access: 'subscriber-only',
  };
}

function buildPublicMedicalRecordDeletionAcknowledgement({
  message = 'Record deleted successfully',
  audit_receipt = null,
} = {}) {
  return {
    message,
    audit_receipt,
  };
}

module.exports = {
  buildPublicMedicalRecordDeletionAcknowledgement,
  buildPublicMedicalRecordEncryptionStatusResponse,
  buildPublicMedicalRecordListResponse,
  buildPublicMedicalRecordResponse,
  buildPublicMedicalRecordVersionEnvelope,
  buildPublicMedicalRecordVersionListResponse,
};
