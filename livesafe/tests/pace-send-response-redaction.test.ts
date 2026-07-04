import { describe, expect, it } from "vitest";

const {
  buildTrusteeInvitationSendResponse,
} = require("../server/routes/pace.js");

describe("P.A.C.E. invitation send response redaction", () => {
  it("returns a bounded response without trustee identity, invitation tokens, or delivery internals", () => {
    const response = buildTrusteeInvitationSendResponse({
      trusteeId: 17,
      role: "primary",
      roleInfo: {
        name: "Primary",
        letter: "P",
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
      id: 17,
      role: "primary",
      role_name: "Primary",
      role_letter: "P",
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
      message: "P.A.C.E. invitation delivery updated for the Primary role.",
    });
    expect(JSON.stringify(response)).not.toContain("secret-token");
    expect(JSON.stringify(response)).not.toContain("invitation_url");
    expect(JSON.stringify(response)).not.toContain("provider_message_id");
  });
});
