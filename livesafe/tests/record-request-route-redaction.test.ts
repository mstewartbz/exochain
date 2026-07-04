import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("record request route redaction wiring", () => {
  it("routes record-request lifecycle responses through the bounded helper", () => {
    const recordsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/records.js"),
      "utf8",
    );
    const earlyRequestsBlock = recordsRoute.slice(
      recordsRoute.indexOf("router.get('/requests', authMiddleware"),
      recordsRoute.indexOf("// GET /api/records/:id - Get a single record by ID"),
    );

    expect(recordsRoute).toContain("buildPublicRecordRequestResponse");
    expect(recordsRoute).toContain(
      "res.status(201).json(buildPublicRecordRequestResponse(request));",
    );
    expect(recordsRoute).toContain(
      "res.json(result.rows.map(buildPublicRecordRequestResponse));",
    );
    expect(earlyRequestsBlock).toContain(
      "res.json(result.rows.map(buildPublicRecordRequestResponse));",
    );
    expect(recordsRoute).not.toContain("res.status(201).json(request);");
    expect(recordsRoute).not.toContain("res.json(updated);");
    expect(earlyRequestsBlock).not.toContain("res.json(result.rows);");
  });
});
