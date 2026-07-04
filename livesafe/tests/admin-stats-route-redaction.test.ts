const fs = require("node:fs");
const path = require("node:path");

describe("admin stats route redaction wiring", () => {
  it("routes platform-admin stats through a bounded helper instead of hand-built payloads", () => {
    const adminRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/admin.js"),
      "utf8",
    );

    expect(adminRoute).toContain("buildAdminStatsResponse({");
    expect(adminRoute).not.toContain("res.json({\n      subscribers: {");
    expect(adminRoute).not.toContain("subscribers: {\n        total: parseInt(subResult.rows[0].total),");
  });
});
