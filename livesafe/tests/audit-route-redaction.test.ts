import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("audit route redaction wiring", () => {
  it("routes audit reads through bounded helpers instead of raw audit rows", () => {
    const auditRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/audit.js"),
      "utf8",
    );

    expect(auditRoute).toContain("buildAuditTrailResponse(result.rows)");
    expect(auditRoute).toContain("buildAuditEventResponse(result.rows[0])");
    expect(auditRoute).not.toContain("SELECT * FROM audit_receipts");
    expect(auditRoute).not.toContain("res.json(result.rows);");
    expect(auditRoute).not.toContain("res.json(result.rows[0]);");
  });
});
