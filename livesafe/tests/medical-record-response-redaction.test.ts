import { describe, expect, it } from "vitest";

const {
  buildPublicMedicalRecordDeletionAcknowledgement,
  buildPublicMedicalRecordEncryptionStatusResponse,
  buildPublicMedicalRecordResponse,
  buildPublicMedicalRecordVersionEnvelope,
  buildPublicMedicalRecordVersionListResponse,
} = require("../server/utils/medical-record-response.js");

describe("medical record response redaction", () => {
  it("returns bounded medical-record metadata without raw storage internals", () => {
    const response = buildPublicMedicalRecordResponse({
      id: 42,
      subscriber_id: 7,
      title: "ER discharge summary",
      record_type: "discharge_summary",
      category: "hospital",
      file_path: "records/subscriber-7/discharge-summary.pdf",
      file_format: "application/pdf",
      file_size: 204800,
      extracted_data: {
        format: "FHIR R4",
        summary: { problems_count: 2 },
      },
      annotation: "Need to discuss potassium levels",
      encrypted: true,
      visibility: "specific_providers",
      visibility_providers: [18, 21],
      version: 3,
      parent_record_id: 11,
      file_hash: "abc123hash",
      created_at: "2026-06-06T11:30:00.000Z",
      updated_at: "2026-06-06T11:32:00.000Z",
    });

    expect(response).toEqual({
      id: 42,
      title: "ER discharge summary",
      record_type: "discharge_summary",
      category: "hospital",
      file_format: "application/pdf",
      file_size: 204800,
      extracted_data: {
        format: "FHIR R4",
        summary: { problems_count: 2 },
      },
      annotation: "Need to discuss potassium levels",
      encrypted: true,
      visibility: "specific_providers",
      visibility_providers: [18, 21],
      version: 3,
      version_number: 3,
      download_available: true,
      created_at: "2026-06-06T11:30:00.000Z",
      updated_at: "2026-06-06T11:32:00.000Z",
    });
    expect(response).not.toHaveProperty("subscriber_id");
    expect(response).not.toHaveProperty("file_path");
    expect(response).not.toHaveProperty("parent_record_id");
    expect(response).not.toHaveProperty("file_hash");
  });

  it("returns bounded version summaries without exposing internal chain ids", () => {
    const versions = buildPublicMedicalRecordVersionListResponse([
      {
        id: 42,
        subscriber_id: 7,
        title: "Discharge summary v3",
        record_type: "discharge_summary",
        category: "hospital",
        file_path: "records/subscriber-7/discharge-summary-v3.pdf",
        file_format: "application/pdf",
        file_size: 220000,
        version: 3,
        parent_record_id: 11,
        created_at: "2026-06-06T11:30:00.000Z",
        updated_at: "2026-06-06T11:32:00.000Z",
      },
    ]);

    expect(versions).toEqual([
      {
        id: 42,
        title: "Discharge summary v3",
        record_type: "discharge_summary",
        category: "hospital",
        file_format: "application/pdf",
        file_size: 220000,
        extracted_data: null,
        annotation: null,
        encrypted: false,
        visibility: null,
        visibility_providers: null,
        version: 3,
        version_number: 3,
        download_available: true,
        created_at: "2026-06-06T11:30:00.000Z",
        updated_at: "2026-06-06T11:32:00.000Z",
      },
    ]);
    expect(versions[0]).not.toHaveProperty("subscriber_id");
    expect(versions[0]).not.toHaveProperty("file_path");
    expect(versions[0]).not.toHaveProperty("parent_record_id");
  });

  it("returns bounded version-history envelopes without leaking chain internals", () => {
    const response = buildPublicMedicalRecordVersionEnvelope([
      {
        id: 42,
        title: "Discharge summary v3",
        record_type: "discharge_summary",
        category: "hospital",
        file_path: "records/subscriber-7/discharge-summary-v3.pdf",
        file_format: "application/pdf",
        file_size: 220000,
        version: 3,
        parent_record_id: 11,
        created_at: "2026-06-06T11:30:00.000Z",
        updated_at: "2026-06-06T11:32:00.000Z",
      },
    ]);

    expect(response).toEqual({
      versions: [
        {
          id: 42,
          title: "Discharge summary v3",
          record_type: "discharge_summary",
          category: "hospital",
          file_format: "application/pdf",
          file_size: 220000,
          extracted_data: null,
          annotation: null,
          encrypted: false,
          visibility: null,
          visibility_providers: null,
          version: 3,
          version_number: 3,
          download_available: true,
          created_at: "2026-06-06T11:30:00.000Z",
          updated_at: "2026-06-06T11:32:00.000Z",
        },
      ],
      total_versions: 1,
    });
    expect(response.versions[0]).not.toHaveProperty("parent_record_id");
    expect(response.versions[0]).not.toHaveProperty("file_path");
  });

  it("returns bounded encryption-status metadata without leaking storage paths", () => {
    const response = buildPublicMedicalRecordEncryptionStatusResponse({
      id: 42,
      title: "ER discharge summary",
      encrypted: true,
      file_path: "records/subscriber-7/discharge-summary.pdf",
      subscriber_id: 7,
    });

    expect(response).toEqual({
      record_id: 42,
      title: "ER discharge summary",
      encrypted: true,
      algorithm: "AES-256-GCM",
      key_type: "subscriber-derived-PBKDF2",
      access: "subscriber-only",
    });
    expect(response).not.toHaveProperty("file_path");
    expect(response).not.toHaveProperty("subscriber_id");
  });

  it("returns a bounded record-deletion acknowledgement without raw audit internals", () => {
    const response = buildPublicMedicalRecordDeletionAcknowledgement({
      message: "Record deleted successfully",
      audit_receipt: "receipt-123",
      deleted_at: "2026-06-07T17:48:00.000Z",
      record_id: 42,
      subscriber_id: 7,
    });

    expect(response).toEqual({
      message: "Record deleted successfully",
      audit_receipt: "receipt-123",
    });
    expect(response).not.toHaveProperty("deleted_at");
    expect(response).not.toHaveProperty("record_id");
    expect(response).not.toHaveProperty("subscriber_id");
  });
});
