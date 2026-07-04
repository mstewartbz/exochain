const {
  buildPublicEmailVerificationResponse,
  buildPublicPhoneVerificationRequestResponse,
  buildPublicPhoneVerificationConfirmResponse,
} = require("../server/utils/verification-response.js");

describe("verification response redaction", () => {
  it("builds a bounded email verification acknowledgement without echoing the raw email", () => {
    const response = buildPublicEmailVerificationResponse({
      email: "ada@example.com",
      alreadyVerified: false,
    });

    expect(response).toEqual({
      message: "Email verified successfully",
      verified: true,
      already_verified: false,
      verification_target: "a***@example.com",
    });
    expect(response).not.toHaveProperty("email");
  });

  it("builds a bounded phone verification request acknowledgement without echoing the raw phone", () => {
    const response = buildPublicPhoneVerificationRequestResponse({
      phone: "+1 (555) 222-9876",
      expiresAt: "2026-06-07T03:45:00.000Z",
      devCode: "123456",
    });

    expect(response).toEqual({
      message: "Verification code sent",
      verification_target: "***-***-9876",
      expires_at: "2026-06-07T03:45:00.000Z",
      dev_code: "123456",
    });
    expect(response).not.toHaveProperty("phone");
  });

  it("builds a bounded phone verification confirmation acknowledgement without echoing the raw phone", () => {
    const response = buildPublicPhoneVerificationConfirmResponse({
      phone: "+1 (555) 222-9876",
      alreadyVerified: true,
      identityCorePointsAwarded: 0,
    });

    expect(response).toEqual({
      message: "Phone already verified",
      verified: true,
      already_verified: true,
      verification_target: "***-***-9876",
      identity_core_points_awarded: 0,
    });
    expect(response).not.toHaveProperty("phone");
  });
});
