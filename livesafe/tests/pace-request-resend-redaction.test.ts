import { describe, expect, it } from "vitest";

const {
  buildTrusteeInvitationResendResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. invitation resend acknowledgement redaction", () => {
  it("returns a bounded acknowledgement without subscriber or trustee identity fields", () => {
    const response = buildTrusteeInvitationResendResponse();

    expect(response).toEqual({
      success: true,
      message:
        "A notification has been sent to the subscriber asking them to resend your invitation.",
      code: "PACE_RESEND_REQUEST_RECORDED",
    });
  });
});
