import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("subscriber auth route redaction wiring", () => {
  it("routes subscriber auth responses through bounded helpers", () => {
    const authRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/auth.js"),
      "utf8",
    );

    const registerStart = authRoute.indexOf("router.post('/register'");
    const loginStart = authRoute.indexOf("router.post('/login'");
    const meStart = authRoute.indexOf("router.get('/me'");
    const trusteeStart = authRoute.indexOf("// =============================================================================\n// TRUSTEE AUTH ROUTES");

    const registerBlock = authRoute.slice(registerStart, loginStart);
    const loginBlock = authRoute.slice(loginStart, meStart);
    const meBlock = authRoute.slice(meStart, trusteeStart);

    expect(authRoute).toContain("buildPublicSubscriberAuthResponse");
    expect(authRoute).toContain("buildPublicSubscriberAuthSessionResponse");
    expect(registerBlock).toContain("buildPublicSubscriberAuthSessionResponse({");
    expect(loginBlock).toContain("buildPublicSubscriberAuthSessionResponse({");
    expect(meBlock).toContain("buildPublicSubscriberAuthResponse(user)");
    expect(meBlock).toContain("buildPublicSubscriberAuthResponse({");
    expect(registerBlock).not.toContain("user: {\n        ...result.rows[0],");
    expect(loginBlock).not.toContain("user: {\n        id: user.id,");
    expect(meBlock).not.toContain("res.json({\n      ...userData,");
    expect(meBlock).not.toContain("created_at");
  });
});
