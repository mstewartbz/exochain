const fs = require("node:fs");
const path = require("node:path");

describe("admin agency dashboard payload alignment", () => {
  it("renders bounded agency fields and uses the redacted mutation count", () => {
    const dashboardSource = fs.readFileSync(
      path.join(process.cwd(), "client/src/pages/AdminDashboard.jsx"),
      "utf8",
    );

    expect(dashboardSource).not.toContain("agency.admin_email");
    expect(dashboardSource).not.toContain("res.data.reactivated_responders");
    expect(dashboardSource).toContain("res.data.affected_responders");
  });
});
