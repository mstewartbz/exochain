import { describe, expect, it } from "vitest";

const {
  buildTrusteeInvitationSendResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. trustee creation response redaction", () => {
  it("reuses the bounded invitation-delivery shape for trustee nomination responses", () => {
    const response = buildTrusteeInvitationSendResponse({
      trusteeId: 42,
      role: "alternate",
      roleInfo: {
        name: "Alternate",
        letter: "A",
      },
      status: "pending",
      delivery: {
        link: {
          status: "available",
          invitation_url: "https://example.test/trustee/accept?token=secret-token",
        },
        email: {
          status: "sent",
          provider_message_id: "email-123",
        },
        sms: {
          status: "failed",
          reason: "notification_delivery_failed",
          provider_message_id: "sms-123",
        },
      },
      messageBodyStored: false,
    });

    expect(response).toEqual({
      id: 42,
      role: "alternate",
      role_name: "Alternate",
      role_letter: "A",
      status: "pending",
      delivery: {
        link: { status: "available" },
        email: { status: "sent" },
        sms: {
          status: "failed",
          reason: "notification_delivery_failed",
        },
      },
      message_body_stored: false,
      code: "PACE_INVITATION_SENT",
      message: "P.A.C.E. invitation delivery updated for the Alternate role.",
    });
    expect(JSON.stringify(response)).not.toContain("secret-token");
    expect(JSON.stringify(response)).not.toContain("invitation_url");
    expect(JSON.stringify(response)).not.toContain("provider_message_id");
    expect(JSON.stringify(response)).not.toContain("alternate@example.com");
    expect(JSON.stringify(response)).not.toContain("invite_phone");
  });
});
