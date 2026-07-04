import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("trustee auth route redaction wiring", () => {
  it("routes trustee auth responses through bounded helpers", () => {
    const authRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/auth.js"),
      "utf8",
    );

    const trusteeLoginStart = authRoute.indexOf("router.post('/trustee/login'");
    const trusteeMeStart = authRoute.indexOf("router.get('/trustee/me'");
    const responderStart = authRoute.indexOf(
      "// =============================================================================\n// RESPONDER AUTH ROUTES",
    );

    const trusteeLoginBlock = authRoute.slice(trusteeLoginStart, trusteeMeStart);
    const trusteeMeBlock = authRoute.slice(trusteeMeStart, responderStart);

    expect(authRoute).toContain("buildPublicTrusteeAuthResponse");
    expect(authRoute).toContain("buildPublicTrusteeAuthSessionResponse");
    expect(authRoute).toContain("buildPublicTrusteeProfileResponse");
    expect(trusteeLoginBlock).toContain("buildPublicTrusteeAuthSessionResponse({");
    expect(trusteeMeBlock).toContain("buildPublicTrusteeProfileResponse({");
    expect(trusteeLoginBlock).not.toContain("user: {");
    expect(trusteeMeBlock).not.toContain("subscriber_did:");
    expect(trusteeMeBlock).not.toContain("res.json({\n      id: first.id,");
  });
});
