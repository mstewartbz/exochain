import { describe, expect, it } from "vitest";

const {
  buildTrusteeNominationValidationErrorResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. trustee nomination validation redaction", () => {
  it("redacts invalid trustee email details from nomination errors", () => {
    const response = buildTrusteeNominationValidationErrorResponse({
      type: "invalid_email",
      email: "Casey.Bad@Example",
    });

    expect(response).toEqual({
      status: 400,
      body: {
        error: "Trustee email is required and must be valid.",
        code: "INVALID_TRUSTEE_EMAIL",
      },
    });
  });

  it("redacts missing SMS phone details from nomination errors", () => {
    const response = buildTrusteeNominationValidationErrorResponse({
      type: "missing_sms_phone",
      email: "casey@example.com",
    });

    expect(response).toEqual({
      status: 400,
      body: {
        error: "SMS trustee invitations require a phone number.",
        code: "PACE_SMS_PHONE_REQUIRED",
      },
    });
  });

  it("redacts duplicate nominee details from nomination conflicts", () => {
    const response = buildTrusteeNominationValidationErrorResponse({
      type: "duplicate_nominee",
      email: "casey@example.com",
      existingRole: "primary",
    });

    expect(response).toEqual({
      status: 409,
      body: {
        error: "This contact is already assigned to a P.A.C.E. role for this subscriber.",
        code: "PACE_DUPLICATE_CONTACT_ROLE",
      },
    });
  });
});
