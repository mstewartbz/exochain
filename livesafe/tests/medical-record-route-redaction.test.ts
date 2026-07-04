import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("medical record route redaction wiring", () => {
  it("routes medical-record responses through the bounded helper", () => {
    const recordsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/records.js"),
      "utf8",
    );
    const rootListBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/', authMiddleware"),
      recordsRoute.indexOf("// GET /api/records/providers"),
    );
    const detailBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/:id', authMiddleware"),
      recordsRoute.indexOf("// GET /api/records/:id/versions"),
    );
    const versionsBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/:id/versions', authMiddleware"),
      recordsRoute.indexOf("// GET /api/records/:id/download"),
    );
    const visibilityBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.patch('/:id/visibility', authMiddleware"),
      recordsRoute.indexOf("// PATCH /api/records/:id - Update annotation"),
    );
    const updateBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.patch('/:id', authMiddleware"),
      recordsRoute.indexOf("// DELETE /api/records/:id - Delete a record"),
    );
    const deleteBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.delete('/:id', authMiddleware"),
      recordsRoute.indexOf("// POST /api/records/request"),
    );

    expect(recordsRoute).toContain("buildPublicMedicalRecordResponse");
    expect(recordsRoute).toContain("buildPublicMedicalRecordDeletionAcknowledgement");
    expect(recordsRoute).toContain("buildPublicMedicalRecordListResponse");
    expect(recordsRoute).toContain("buildPublicMedicalRecordVersionEnvelope");
    expect(recordsRoute).toContain("buildPublicMedicalRecordVersionListResponse");
    expect(recordsRoute).toContain("buildPublicMedicalRecordEncryptionStatusResponse");
    expect(recordsRoute).toContain("record: buildPublicMedicalRecordResponse(result.rows[0])");
    expect(recordsRoute).toContain("return res.json(buildPublicMedicalRecordListResponse(result.rows));");
    expect(recordsRoute).toContain("res.json(buildPublicMedicalRecordResponse(result.rows[0]));");
    expect(recordsRoute).toContain("res.json(buildPublicMedicalRecordVersionEnvelope(versions.rows));");
    expect(recordsRoute).toContain("res.json(buildPublicMedicalRecordEncryptionStatusResponse(record));");
    expect(deleteBlock).toContain("buildPublicMedicalRecordDeletionAcknowledgement({");
    expect(recordsRoute).not.toContain("record: result.rows[0],");
    expect(rootListBlock).not.toContain("data: result.rows");
    expect(rootListBlock).not.toContain("res.json(result.rows);");
    expect(detailBlock).not.toContain("res.json(result.rows[0]);");
    expect(versionsBlock).not.toContain("versions: buildPublicMedicalRecordVersionListResponse(versions.rows)");
    expect(versionsBlock).not.toContain("versions: versions.rows,");
    expect(versionsBlock).not.toContain("total_versions: versions.rows.length");
    expect(versionsBlock).not.toContain("algorithm: record.encrypted ? 'AES-256-GCM' : null");
    expect(versionsBlock).not.toContain("key_type: record.encrypted ? 'subscriber-derived-PBKDF2' : null");
    expect(visibilityBlock).not.toContain("res.json(result.rows[0]);");
    expect(updateBlock).not.toContain("res.json(result.rows[0]);");
    expect(deleteBlock).not.toContain("res.json({");
    expect(deleteBlock).not.toContain("deleted_at,\n      })");
  });

  it("routes record-provider directory responses through the bounded helper", () => {
    const recordsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/records.js"),
      "utf8",
    );
    const providersBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/providers', authMiddleware"),
      recordsRoute.indexOf("// GET /api/records/requests"),
    );

    expect(recordsRoute).toContain("buildPublicRecordProviderListResponse");
    expect(recordsRoute).toContain(
      "res.json(buildPublicRecordProviderListResponse(result.rows));",
    );
    expect(providersBlock).toContain(
      "res.json(buildPublicRecordProviderListResponse(result.rows));",
    );
    expect(providersBlock).not.toContain("res.json(result.rows);");
    expect(recordsRoute.match(/router\.get\('\/providers', authMiddleware/g)?.length).toBe(1);
  });
});
