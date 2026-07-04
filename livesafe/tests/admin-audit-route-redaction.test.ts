const fs = require("node:fs");
const path = require("node:path");

describe("admin audit route redaction wiring", () => {
  it("routes admin audit reads through bounded helpers instead of raw audit rows", () => {
    const adminRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/admin.js"),
      "utf8",
    );

    expect(adminRoute).toContain("buildAdminAuditTrailResponse(result.rows, {");
    expect(adminRoute).not.toContain("records: result.rows,");
    expect(adminRoute).not.toContain("res.json({\n      records: result.rows,");
    expect(adminRoute).not.toContain("SELECT ar.*, s.email as subscriber_email");
  });
});
