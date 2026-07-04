const {
  buildAdminAgencyResponderListResponse,
  buildAdminResponderResponse,
  buildAdminResponderListResponse,
  buildAdminResponderToggleResponse,
} = require("../server/utils/admin-responder-response.js");

describe("admin responder response redaction", () => {
  const responderRow = {
    id: 19,
    did: "did:exo:responder:private",
    email: "responder@example.com",
    role: "paramedic",
    certification: "EMT-P",
    is_military: false,
    is_active: true,
    agency_id: 88,
    created_at: "2026-06-06T12:00:00.000Z",
  };

  it("builds a bounded admin responder payload without DID or agency bindings", () => {
    expect(buildAdminResponderResponse(responderRow)).toEqual({
      id: 19,
      email: "responder@example.com",
      role: "paramedic",
      certification: "EMT-P",
      is_military: false,
      is_active: true,
      created_at: "2026-06-06T12:00:00.000Z",
    });
  });

  it("builds a bounded admin responder list payload without raw responder rows", () => {
    expect(buildAdminResponderListResponse([responderRow])).toEqual({
      responders: [
        {
          id: 19,
          email: "responder@example.com",
          role: "paramedic",
          certification: "EMT-P",
          is_military: false,
          is_active: true,
          created_at: "2026-06-06T12:00:00.000Z",
        },
      ],
      total: 1,
      active: 1,
    });
  });

  it("builds a bounded agency responder roster payload without raw agency rows", () => {
    expect(
      buildAdminAgencyResponderListResponse({
        agency: {
          id: 88,
          name: "Wake EMS",
          admin_email: "chief@example.com",
        },
        responders: [responderRow],
      }),
    ).toEqual({
      agency: {
        id: 88,
        name: "Wake EMS",
      },
      responders: [
        {
          id: 19,
          email: "responder@example.com",
          role: "paramedic",
          certification: "EMT-P",
          is_military: false,
          is_active: true,
          created_at: "2026-06-06T12:00:00.000Z",
        },
      ],
      total: 1,
      active: 1,
    });
  });

  it("builds a bounded responder toggle acknowledgement without raw route composition", () => {
    expect(
      buildAdminResponderToggleResponse({
        is_active: false,
        responder: responderRow,
      }),
    ).toEqual({
      message: "Responder deactivated successfully",
      responder: {
        id: 19,
        email: "responder@example.com",
        role: "paramedic",
        certification: "EMT-P",
        is_military: false,
        is_active: true,
        created_at: "2026-06-06T12:00:00.000Z",
      },
    });
  });
});
