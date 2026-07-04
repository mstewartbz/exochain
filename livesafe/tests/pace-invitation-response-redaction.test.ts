import { describe, expect, it } from "vitest";

const {
  buildTrusteeInvitationValidateResponse,
  buildTrusteeInvitationDeclineResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. invitation response redaction", () => {
  it("redacts trustee and subscriber identity from unauthenticated invitation validation responses", () => {
    const response = buildTrusteeInvitationValidateResponse({
      invitationId: 42,
      role: "primary",
      roleInfo: {
        name: "Primary",
        letter: "P",
        description: "Primary trustee",
        responsibilities: ["Coordinate initial response"],
      },
      invitationEmail: "trustee@example.com",
      subscriberName: "Taylor Subscriber",
    });

    expect(response).toEqual({
      id: 42,
      role: "primary",
      role_name: "Primary",
      role_letter: "P",
      role_description: "Primary trustee",
      role_responsibilities: ["Coordinate initial response"],
      code: "PACE_INVITATION_VALID",
      message: "This invitation is valid and can be accepted for the Primary role.",
    });
  });

  it("redacts trustee and subscriber identity from unauthenticated invitation decline acknowledgements", () => {
    const response = buildTrusteeInvitationDeclineResponse({
      role: "alternate",
      roleInfo: {
        name: "Alternate",
      },
      subscriberName: "Taylor Subscriber",
    });

    expect(response).toEqual({
      success: true,
      status: "declined",
      role: "alternate",
      role_name: "Alternate",
      code: "PACE_INVITATION_DECLINED",
      message: "You declined this P.A.C.E. invitation.",
    });
  });
});
