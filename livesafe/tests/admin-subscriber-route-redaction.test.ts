const fs = require("node:fs");
const path = require("node:path");

describe("admin subscriber route redaction wiring", () => {
  it("routes admin subscriber list, detail, and update responses through bounded helpers", () => {
    const adminRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/admin.js"),
      "utf8",
    );

    expect(adminRoute).toContain("buildAdminSubscriberListResponse(result.rows, {");
    expect(adminRoute).toContain("res.json(buildAdminSubscriberResponse(result.rows[0]));");
    expect(adminRoute).not.toContain("SELECT id, did, email, first_name, last_name, role, email_verified, created_at, updated_at");
    expect(adminRoute).not.toContain("subscribers: result.rows,");
    expect(adminRoute).not.toContain("res.json(result.rows[0]);");
    expect(adminRoute).not.toContain("RETURNING id, did, email, first_name, last_name, role, email_verified, created_at, updated_at");
  });
});
