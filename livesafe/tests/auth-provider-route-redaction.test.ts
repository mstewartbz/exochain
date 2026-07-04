import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("provider auth route redaction wiring", () => {
  it("routes provider auth responses through bounded helpers", () => {
    const authRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/auth.js"),
      "utf8",
    );

    const providerNpiLookupStart = authRoute.indexOf("router.get('/provider/npi-lookup/:npi'");
    const providerRegisterStart = authRoute.indexOf("router.post('/provider/register'");
    const providerLoginStart = authRoute.indexOf("router.post('/provider/login'");
    const providerMeStart = authRoute.indexOf("router.get('/provider/me'");
    const moduleExportStart = authRoute.indexOf("module.exports = router;");

    const providerNpiLookupBlock = authRoute.slice(
      providerNpiLookupStart,
      providerRegisterStart,
    );
    const providerRegisterBlock = authRoute.slice(
      providerRegisterStart,
      providerLoginStart,
    );
    const providerLoginBlock = authRoute.slice(
      providerLoginStart,
      providerMeStart,
    );
    const providerMeBlock = authRoute.slice(
      providerMeStart,
      moduleExportStart,
    );

    expect(authRoute).toContain("buildPublicProviderAuthResponse");
    expect(authRoute).toContain("buildPublicProviderAuthSessionResponse");
    expect(authRoute).toContain("buildPublicProviderAuthProfileResponse");
    expect(authRoute).toContain("buildPublicProviderNpiLookupResponse");
    expect(providerNpiLookupBlock).toContain("res.json(buildPublicProviderNpiLookupResponse(result));");
    expect(providerNpiLookupBlock).not.toContain("first_name: result.first_name");
    expect(providerNpiLookupBlock).not.toContain("last_name: result.last_name");
    expect(providerNpiLookupBlock).not.toContain("addresses: result.addresses");
    expect(providerRegisterBlock).toContain("buildPublicProviderAuthSessionResponse({");
    expect(providerLoginBlock).toContain("buildPublicProviderAuthSessionResponse({");
    expect(providerMeBlock).toContain("buildPublicProviderAuthProfileResponse({");
    expect(providerRegisterBlock).not.toContain("user: {\n        ...provider,");
    expect(providerLoginBlock).not.toContain("user: {\n        id: provider.id,");
    expect(providerMeBlock).not.toContain("res.json({\n      ...provider,");
    expect(providerMeBlock).not.toContain("subscriber_id:");
    expect(providerMeBlock).not.toContain("created_at FROM providers");
  });
});
