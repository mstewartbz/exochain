const fs = require("node:fs");
const path = require("node:path");

describe("admin responder route redaction wiring", () => {
  it("routes admin responder list and toggle responses through bounded helpers", () => {
    const adminRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/admin.js"),
      "utf8",
    );

    expect(adminRoute).toContain("buildAdminResponderListResponse,");
    expect(adminRoute).toContain("buildAdminAgencyResponderListResponse({");
    expect(adminRoute).toContain("buildAdminResponderResponse,");
    expect(adminRoute).toContain("buildAdminResponderToggleResponse({");
    expect(adminRoute).not.toContain("SELECT id, did, email, role, certification, is_military, is_active, created_at");
    expect(adminRoute).not.toContain("responder: buildAdminResponderResponse(responder),");
    expect(adminRoute).not.toContain("RETURNING id, email, is_active, agency_id");
    expect(adminRoute).not.toContain("agency_id: parseInt(id),");
    expect(adminRoute).not.toContain("agency_name: agencyResult.rows[0].name,");
    expect(adminRoute).not.toContain("message: `Responder ${is_active ? 'activated' : 'deactivated'} successfully`,");
  });
});
