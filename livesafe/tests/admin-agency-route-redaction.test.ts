const fs = require("node:fs");
const path = require("node:path");

describe("admin agency route redaction wiring", () => {
  it("routes admin agency list and mutation responses through bounded helpers", () => {
    const adminRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/admin.js"),
      "utf8",
    );

    expect(adminRoute).toContain("buildAdminAgencyListResponse(result.rows)");
    expect(adminRoute).toContain("buildAdminAgencyMutationResponse({");
    expect(adminRoute).not.toContain("SELECT a.id, a.name, a.type, a.admin_email, a.is_active, a.created_at,");
    expect(adminRoute).not.toContain("UPDATE responders SET is_active = FALSE WHERE agency_id = $1 AND is_active = TRUE RETURNING id, email");
    expect(adminRoute).not.toContain("deactivated_responders: deactivatedCount,");
    expect(adminRoute).not.toContain("reactivated_responders: responderResult.rows.length,");
    expect(adminRoute).not.toContain("responders: responderResult.rows,");
    expect(adminRoute).not.toContain("admin_email");
  });
});
