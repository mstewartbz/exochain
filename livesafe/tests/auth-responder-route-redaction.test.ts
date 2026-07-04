import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("responder auth route redaction wiring", () => {
  it("routes responder and agency auth responses through bounded helpers", () => {
    const authRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/auth.js"),
      "utf8",
    );

    const responderRegisterStart = authRoute.indexOf(
      "router.post('/responder/register'",
    );
    const responderLoginStart = authRoute.indexOf(
      "router.post('/responder/login'",
    );
    const responderMeStart = authRoute.indexOf("router.get('/responder/me'");
    const agencyRegisterStart = authRoute.indexOf("router.post('/agency/register'");
    const agenciesListStart = authRoute.indexOf("router.get('/agencies'");

    const responderRegisterBlock = authRoute.slice(
      responderRegisterStart,
      responderLoginStart,
    );
    const responderLoginBlock = authRoute.slice(
      responderLoginStart,
      responderMeStart,
    );
    const responderMeBlock = authRoute.slice(
      responderMeStart,
      agencyRegisterStart,
    );
    const agencyRegisterBlock = authRoute.slice(
      agencyRegisterStart,
      agenciesListStart,
    );
    const agenciesListBlock = authRoute.slice(agenciesListStart);

    expect(authRoute).toContain("buildPublicResponderAuthResponse(");
    expect(authRoute).toContain("buildPublicResponderAuthSessionResponse(");
    expect(authRoute).toContain("buildPublicAgencyRegistrationSessionResponse(");
    expect(authRoute).toContain("buildPublicAgencyDirectoryEntry,");
    expect(responderRegisterBlock).not.toContain("...responder");
    expect(responderRegisterBlock).not.toContain("agency_id:");
    expect(responderRegisterBlock).toContain(
      "buildPublicResponderAuthSessionResponse({",
    );
    expect(responderLoginBlock).toContain(
      "buildPublicResponderAuthSessionResponse({",
    );
    expect(responderLoginBlock).not.toContain("user: buildPublicResponderAuthResponse(responder),");
    expect(responderLoginBlock).not.toContain("agency_id:");
    expect(responderMeBlock).not.toContain("...responder");
    expect(agencyRegisterBlock).toContain(
      "buildPublicAgencyRegistrationSessionResponse({",
    );
    expect(agencyRegisterBlock).not.toContain("...registrationResponse");
    expect(agencyRegisterBlock).not.toContain("admin_email: agency.admin_email");
    expect(agencyRegisterBlock).not.toContain("...admin");
    expect(agenciesListBlock).toContain(
      "res.json(result.rows.map(buildPublicAgencyDirectoryEntry));",
    );
    expect(agenciesListBlock).not.toContain("res.json(result.rows);");
  });
});
