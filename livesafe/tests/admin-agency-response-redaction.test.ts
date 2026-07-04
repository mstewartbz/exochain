const {
  buildAdminAgencyListResponse,
  buildAdminAgencyMutationResponse,
  buildAdminAgencyResponse,
} = require("../server/utils/admin-agency-response.js");

describe("admin agency response redaction", () => {
  const agencyRow = {
    id: 12,
    name: "Wake EMS",
    type: "ems",
    admin_email: "chief@example.com",
    is_active: true,
    created_at: "2026-06-06T13:00:00.000Z",
    responder_count: "4",
    active_responders: "3",
  };

  it("builds a bounded admin agency payload without admin email", () => {
    expect(buildAdminAgencyResponse(agencyRow)).toEqual({
      id: 12,
      name: "Wake EMS",
      type: "ems",
      is_active: true,
      created_at: "2026-06-06T13:00:00.000Z",
      responder_count: 4,
      active_responders: 3,
    });
  });

  it("builds a bounded admin agency list payload without raw agency rows", () => {
    expect(buildAdminAgencyListResponse([agencyRow])).toEqual({
      agencies: [
        {
          id: 12,
          name: "Wake EMS",
          type: "ems",
          is_active: true,
          created_at: "2026-06-06T13:00:00.000Z",
          responder_count: 4,
          active_responders: 3,
        },
      ],
      total: 1,
    });
  });

  it("builds bounded admin agency mutation responses without raw responder rows", () => {
    expect(
      buildAdminAgencyMutationResponse({
        message: 'Agency "Wake EMS" deactivated successfully',
        agency: agencyRow,
        affected_responders: 2,
      }),
    ).toEqual({
      message: 'Agency "Wake EMS" deactivated successfully',
      agency: {
        id: 12,
        name: "Wake EMS",
        type: "ems",
        is_active: true,
        created_at: "2026-06-06T13:00:00.000Z",
        responder_count: 4,
        active_responders: 3,
      },
      affected_responders: 2,
    });
  });
});
