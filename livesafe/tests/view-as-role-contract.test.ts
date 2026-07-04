import { readFileSync } from "node:fs";
import path from "node:path";

function read(relativePath: string): string {
  return readFileSync(path.join(process.cwd(), relativePath), "utf8");
}

describe("admin view-as role contract", () => {
  it("issues only short-lived role-view sessions from verified subscriber admins", () => {
    const authRoute = read("server/routes/auth.js");

    expect(authRoute).toContain("const VIEW_AS_ROLES = new Set(['subscriber'])");
    expect(authRoute).toContain("const VIEW_AS_SESSION_EXPIRES_IN = '30m'");
    expect(authRoute).toContain("router.post('/view-as', authMiddleware");
    expect(authRoute).toContain("req.user.view_as_mode");
    expect(authRoute).toContain("admin.role !== 'subscriber_admin' || req.user.role !== 'subscriber_admin'");
    expect(authRoute).toContain("actual_role: admin.role");
    expect(authRoute).toContain("view_as_mode: true");
    expect(authRoute).toContain("sessionExpiresIn: VIEW_AS_SESSION_EXPIRES_IN");
    expect(authRoute).not.toContain("target_subscriber_id");
    expect(authRoute).not.toContain("impersonat");
  });

  it("keeps /auth/me fail-closed for stale or malformed view-as tokens", () => {
    const authRoute = read("server/routes/auth.js");

    expect(authRoute).toContain("if (decoded.view_as_mode)");
    expect(authRoute).toContain("user.role !== 'subscriber_admin'");
    expect(authRoute).toContain("decoded.actual_role !== 'subscriber_admin'");
    expect(authRoute).toContain("!VIEW_AS_ROLES.has(decoded.view_as_role)");
    expect(authRoute).toContain("decoded.role !== decoded.view_as_role");
    expect(authRoute).toContain("return res.status(403).json({ error: 'Invalid view-as session' })");
  });

  it("wires visible enter and exit controls without hiding the simulated role", () => {
    const authContext = read("client/src/context/AuthContext.jsx");
    const navbar = read("client/src/components/Navbar.jsx");
    const adminDashboard = read("client/src/pages/AdminDashboard.jsx");
    const api = read("client/src/services/api.js");

    expect(authContext).toContain("livesafe_admin_token_before_view_as");
    expect(authContext).toContain("startViewAsRole");
    expect(authContext).toContain("stopViewAsRole");
    expect(authContext).toContain("api.post('/auth/view-as'");
    expect(authContext).toContain("isViewingAsRole: Boolean(user?.view_as?.active)");
    expect(api).toContain("localStorage.removeItem('livesafe_admin_token_before_view_as')");
    expect(adminDashboard).toContain("data-testid=\"view-as-subscriber-btn\"");
    expect(adminDashboard).toContain("startViewAsRole('subscriber')");
    expect(navbar).toContain("data-testid=\"view-as-banner\"");
    expect(navbar).toContain("data-testid=\"exit-view-as-btn\"");
    expect(navbar).toContain("stopViewAsRole");
  });
});
