import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("clinical note route redaction wiring", () => {
  it("routes provider clinical-note responses through bounded helper envelopes", () => {
    const recordsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/records.js"),
      "utf8",
    );
    const createBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.post('/clinical-notes'"),
      recordsRoute.indexOf("// GET /api/records/clinical-notes/subscriber"),
    );
    const subscriberListBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/clinical-notes/subscriber'"),
      recordsRoute.indexOf("// GET /api/records/clinical-notes/provider"),
    );
    const providerListBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/clinical-notes/provider'"),
      recordsRoute.indexOf("// PATCH /api/records/clinical-notes/:id/approve"),
    );
    const approveBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.patch('/clinical-notes/:id/approve'"),
      recordsRoute.indexOf("// PATCH /api/records/clinical-notes/:id/reject"),
    );
    const rejectBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.patch('/clinical-notes/:id/reject'"),
      recordsRoute.indexOf("// Handle multer errors"),
    );

    expect(recordsRoute).toContain("buildPublicClinicalNoteResponse");
    expect(recordsRoute).toContain("buildPublicClinicalNoteListResponse");
    expect(recordsRoute).toContain("buildPublicClinicalNoteCreateAcknowledgement");
    expect(recordsRoute).toContain("buildPublicClinicalNoteListEnvelope");
    expect(recordsRoute).toContain("buildPublicClinicalNoteMutationAcknowledgement");
    expect(createBlock).toContain("res.status(201).json(");
    expect(createBlock).toContain("buildPublicClinicalNoteCreateAcknowledgement({");
    expect(subscriberListBlock).toContain("res.json(buildPublicClinicalNoteListEnvelope(notes));");
    expect(providerListBlock).toContain("res.json(buildPublicClinicalNoteListEnvelope(result.rows));");
    expect(approveBlock).toContain("buildPublicClinicalNoteMutationAcknowledgement({");
    expect(rejectBlock).toContain("buildPublicClinicalNoteMutationAcknowledgement({");
    expect(createBlock).not.toContain("note: buildPublicClinicalNoteResponse(note)");
    expect(subscriberListBlock).not.toContain("pending_count: pending.length");
    expect(providerListBlock).not.toContain("res.json({ notes: buildPublicClinicalNoteListResponse(result.rows) });");
    expect(approveBlock).not.toContain("res.json({");
    expect(rejectBlock).not.toContain("note: buildPublicClinicalNoteResponse(result.rows[0])");
  });
});
