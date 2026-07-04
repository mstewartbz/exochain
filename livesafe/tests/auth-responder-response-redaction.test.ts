import { describe, expect, it } from "vitest";

const {
  buildPublicResponderAuthResponse,
  buildPublicResponderAuthSessionResponse,
  buildPublicAgencyDirectoryEntry,
  buildPublicAgencyRegistrationResponse,
  buildPublicAgencyRegistrationSessionResponse,
} = require("../server/utils/auth-responder-response.js");

describe("responder auth response redaction", () => {
  it("builds a bounded responder auth payload without internal agency bindings", () => {
    const response = buildPublicResponderAuthResponse({
      id: 41,
      did: "did:exo:responder:test",
      email: "responder@example.com",
      agency_id: 88,
      agency_name: "Wake EMS",
      agency_type: "ems",
      role: "agency_admin",
      certification: "EMT-P",
      is_military: false,
      created_at: "2026-06-06T21:10:00.000Z",
    });

    expect(response).toEqual({
      id: 41,
      did: "did:exo:responder:test",
      email: "responder@example.com",
      agency_name: "Wake EMS",
      agency_type: "ems",
      role: "agency_admin",
      certification: "EMT-P",
      is_military: false,
      user_type: "responder",
      tier: "free",
    });
    expect(response).not.toHaveProperty("agency_id");
    expect(response).not.toHaveProperty("created_at");
  });

  it("builds a bounded agency registration payload without admin-email echo", () => {
    const response = buildPublicAgencyRegistrationResponse({
      agency: {
        id: 12,
        name: "Wake EMS",
        type: "ems",
        admin_email: "admin@wake.example",
        verified: false,
        created_at: "2026-06-06T21:10:00.000Z",
      },
      admin: {
        id: 41,
        did: "did:exo:responder:test",
        email: "admin@wake.example",
        agency_id: 12,
        agency_name: "Wake EMS",
        agency_type: "ems",
        role: "agency_admin",
        certification: "EMT-P",
        is_military: false,
        created_at: "2026-06-06T21:10:00.000Z",
      },
    });

    expect(response).toEqual({
      agency: {
        id: 12,
        name: "Wake EMS",
        type: "ems",
        verified: false,
      },
      user: {
        id: 41,
        did: "did:exo:responder:test",
        email: "admin@wake.example",
        agency_name: "Wake EMS",
        agency_type: "ems",
        role: "agency_admin",
        certification: "EMT-P",
        is_military: false,
        user_type: "responder",
        tier: "free",
      },
    });
    expect(response.agency).not.toHaveProperty("admin_email");
    expect(response.agency).not.toHaveProperty("created_at");
    expect(response.user).not.toHaveProperty("agency_id");
    expect(response.user).not.toHaveProperty("created_at");
  });

  it("builds a bounded responder auth session payload without internal agency bindings", () => {
    const response = buildPublicResponderAuthSessionResponse({
      user: {
        id: 41,
        did: "did:exo:responder:test",
        email: "responder@example.com",
        agency_id: 88,
        agency_name: "Wake EMS",
        agency_type: "ems",
        role: "agency_admin",
        certification: "EMT-P",
        is_military: false,
        created_at: "2026-06-06T21:10:00.000Z",
      },
      token: "responder-session-token",
    });

    expect(response).toEqual({
      user: {
        id: 41,
        did: "did:exo:responder:test",
        email: "responder@example.com",
        agency_name: "Wake EMS",
        agency_type: "ems",
        role: "agency_admin",
        certification: "EMT-P",
        is_military: false,
        user_type: "responder",
        tier: "free",
      },
      token: "responder-session-token",
    });
    expect(response.user).not.toHaveProperty("agency_id");
    expect(response.user).not.toHaveProperty("created_at");
  });

  it("builds a bounded public agency directory entry without internal timestamps", () => {
    const response = buildPublicAgencyDirectoryEntry({
      id: 12,
      name: "Wake EMS",
      type: "ems",
      verified: true,
      created_at: "2026-06-06T21:10:00.000Z",
      admin_email: "admin@wake.example",
    });

    expect(response).toEqual({
      id: 12,
      name: "Wake EMS",
      type: "ems",
      verified: true,
    });
    expect(response).not.toHaveProperty("created_at");
    expect(response).not.toHaveProperty("admin_email");
  });

  it("builds a bounded agency registration session payload without inline token wrapping", () => {
    const response = buildPublicAgencyRegistrationSessionResponse({
      agency: {
        id: 12,
        name: "Wake EMS",
        type: "ems",
        verified: false,
        admin_email: "admin@wake.example",
      },
      admin: {
        id: 41,
        did: "did:exo:responder:test",
        email: "admin@wake.example",
        agency_id: 12,
        agency_name: "Wake EMS",
        agency_type: "ems",
        role: "agency_admin",
        certification: "EMT-P",
        is_military: false,
      },
      token: "agency-registration-token",
    });

    expect(response).toEqual({
      agency: {
        id: 12,
        name: "Wake EMS",
        type: "ems",
        verified: false,
      },
      user: {
        id: 41,
        did: "did:exo:responder:test",
        email: "admin@wake.example",
        agency_name: "Wake EMS",
        agency_type: "ems",
        role: "agency_admin",
        certification: "EMT-P",
        is_military: false,
        user_type: "responder",
        tier: "free",
      },
      token: "agency-registration-token",
    });
    expect(response.agency).not.toHaveProperty("admin_email");
    expect(response.user).not.toHaveProperty("agency_id");
  });
});
