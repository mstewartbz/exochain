import { afterEach, describe, expect, it, vi } from "vitest";

const {
  buildInvitationUrl,
  createPaceInvitationMessages,
  deliverPaceInvitation,
  normalizeDeliveryChannels,
  resolveInvitationBaseUrl,
} = require("../server/utils/pace-invitations.js");

describe("P.A.C.E. invitation delivery", () => {
  const invitation = {
    subscriberName: "Bob",
    role: "contingent",
    invitationToken: "token-123",
    email: "casey@example.com",
    phone: "+15551234567",
  };

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it("builds a bearer invitation URL from a server-side token", () => {
    expect(
      buildInvitationUrl({
        baseUrl: "https://livesafe-production.up.railway.app/",
        token: "token-123",
      }),
    ).toBe(
      "https://livesafe-production.up.railway.app/trustee/accept?token=token-123",
    );
  });

  it("resolves invitation base URLs from configured frontend and Railway hosts", () => {
    expect(
      resolveInvitationBaseUrl({
        env: {
          CLIENT_URL: "https://app.livesafe.test/",
          RAILWAY_PUBLIC_DOMAIN: "api.livesafe.test",
        },
      }),
    ).toBe("https://app.livesafe.test");

    expect(
      resolveInvitationBaseUrl({
        env: {
          RAILWAY_PUBLIC_DOMAIN: "livesafe-production.up.railway.app",
        },
      }),
    ).toBe("https://livesafe-production.up.railway.app");
  });

  it("uses local browser origins for split frontend/API development without trusting arbitrary production origins", () => {
    const localProxyHeaders: Record<string, string> = {
      host: "127.0.0.1:3001",
      origin: "http://127.0.0.1:3005",
    };
    const localProxyRequest = {
      protocol: "http",
      get(name: string) {
        return localProxyHeaders[name.toLowerCase()];
      },
    };
    expect(resolveInvitationBaseUrl({ req: localProxyRequest, env: {} })).toBe(
      "http://127.0.0.1:3005",
    );

    const localApiHeaders: Record<string, string> = {
      host: "localhost:3001",
    };
    const localApiRequest = {
      protocol: "http",
      get(name: string) {
        return localApiHeaders[name.toLowerCase()];
      },
    };
    expect(resolveInvitationBaseUrl({ req: localApiRequest, env: {} })).toBe(
      "http://localhost:3000",
    );

    const publicHeaders: Record<string, string> = {
      host: "api.livesafe.test",
      origin: "https://evil.test",
    };
    const publicRequest = {
      protocol: "https",
      get(name: string) {
        return publicHeaders[name.toLowerCase()];
      },
    };
    expect(resolveInvitationBaseUrl({ req: publicRequest, env: {} })).toBe(
      "https://api.livesafe.test",
    );
  });

  it("creates channel-specific invite copy without medical facts or referral-bounty framing", () => {
    const messages = createPaceInvitationMessages(invitation, {
      invitationUrl: "https://example.test/trustee/accept?token=token-123",
    });

    expect(messages.sms).toBe(
      "Bob named you as a LiveSafe P.A.C.E. contact. Accept or decline: https://example.test/trustee/accept?token=token-123",
    );
    expect(messages.sms).not.toMatch(/medical|record|allerg|medication/i);
    expect(messages.email.subject).toBe(
      "Bob is asking you to accept a LiveSafe P.A.C.E. role",
    );
    expect(messages.email.text).toContain("This is not a marketing invite.");
    expect(messages.email.text).toContain("Your proposed role: Contingent.");
    expect(messages.email.text).toContain("Accepting this role does not give you Bob's full medical records.");
    expect(messages.email.text).toContain("You can accept, decline, or ask Bob to choose someone else.");
    expect(messages.email.text).not.toMatch(/referral bounty|get four friends|coupon/i);
  });

  it("always exposes copy-link while failing closed for unconfigured email and SMS transports", async () => {
    const result = await deliverPaceInvitation(invitation, {
      appBaseUrl: "https://example.test",
      requestedChannels: ["email", "sms", "link"],
      env: {},
    });

    expect(result.invitation_url).toBe(
      "https://example.test/trustee/accept?token=token-123",
    );
    expect(result.delivery).toMatchObject({
      link: { status: "available" },
      email: { status: "blocked", reason: "email_transport_not_configured" },
      sms: { status: "blocked", reason: "sms_transport_not_configured" },
    });
    expect(result.message_body_stored).toBe(false);
  });

  it("uses configured transports for email and SMS without pretending success", async () => {
    const sendEmail = vi.fn(async () => ({ providerMessageId: "email-1" }));
    const sendSms = vi.fn(async () => ({ providerMessageId: "sms-1" }));

    const result = await deliverPaceInvitation(invitation, {
      appBaseUrl: "https://example.test",
      requestedChannels: ["email", "sms"],
      env: {
        SMTP_HOST: "smtp.example.test",
        SMTP_FROM: "LiveSafe <noreply@example.test>",
        TWILIO_ACCOUNT_SID: "sid",
        TWILIO_AUTH_TOKEN: "secret",
        TWILIO_FROM_NUMBER: "+15557654321",
      },
      transports: {
        sendEmail,
        sendSms,
      },
    });

    expect(sendEmail).toHaveBeenCalledWith(
      expect.objectContaining({
        to: "casey@example.com",
        subject: "Bob is asking you to accept a LiveSafe P.A.C.E. role",
      }),
    );
    expect(sendSms).toHaveBeenCalledWith(
      expect.objectContaining({
        to: "+15551234567",
        body: expect.stringContaining("Accept or decline"),
      }),
    );
    expect(result.delivery.email).toMatchObject({
      status: "sent",
      provider_message_id: "email-1",
    });
    expect(result.delivery.sms).toMatchObject({
      status: "sent",
      provider_message_id: "sms-1",
    });
  });

  it("uses WorkOS invitations when WorkOS credentials are configured and aligns returned tokens", async () => {
    const fetchMock = vi.fn(
      async (_url: string, _init: { body: string }) => ({
        ok: true,
        status: 201,
        json: async () => ({
          id: "invitation_123",
          token: "workos-token-123",
          accept_invitation_url: "https://example.test/invite?invitation_token=workos-token-123",
        }),
      }),
    );
    vi.stubGlobal("fetch", fetchMock);
    const sendSms = vi.fn(async () => ({ providerMessageId: "sms-2" }));

    const result = await deliverPaceInvitation(invitation, {
      appBaseUrl: "https://example.test",
      requestedChannels: ["email", "sms", "link"],
      env: {
        WORKOS_API_KEY: "sk_test",
        WORKOS_CLIENT_ID: "client_test",
        WORKOS_INVITATION_ORGANIZATION_ID: "org_123",
        WORKOS_INVITATION_ROLE_SLUG: "member",
        WORKOS_INVITATION_EXPIRES_IN_DAYS: "7",
        TWILIO_ACCOUNT_SID: "sid",
        TWILIO_AUTH_TOKEN: "secret",
        TWILIO_FROM_NUMBER: "+15557654321",
      },
      transports: {
        sendSms,
      },
    });

    expect(fetchMock).toHaveBeenCalledWith(
      "https://api.workos.com/user_management/invitations",
      expect.objectContaining({
        method: "POST",
        headers: expect.objectContaining({
          Authorization: "Bearer sk_test",
          "Content-Type": "application/json",
        }),
      }),
    );
    const workosCall = fetchMock.mock.calls[0];
    if (!workosCall) {
      throw new Error("WorkOS invitation request was not recorded.");
    }
    const workosRequest = workosCall[1];
    expect(JSON.parse(workosRequest.body)).toEqual({
      email: "casey@example.com",
      expires_in_days: 7,
      organization_id: "org_123",
      role_slug: "member",
    });
    expect(sendSms).toHaveBeenCalledWith(
      expect.objectContaining({
        body: expect.stringContaining("token=workos-token-123"),
      }),
    );
    expect(result.invitation_token).toBe("workos-token-123");
    expect(result.invitation_url).toBe(
      "https://example.test/trustee/accept?token=workos-token-123",
    );
    expect(result.delivery.email).toMatchObject({
      status: "sent",
      provider_message_id: "invitation_123",
    });
    expect(result.delivery.sms).toMatchObject({
      status: "sent",
      provider_message_id: "sms-2",
    });
  });

  it("redacts WorkOS invitation failures without leaking provider response details", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: false,
        status: 401,
        json: async () => ({ message: "api key rejected" }),
      })),
    );

    const result = await deliverPaceInvitation(invitation, {
      appBaseUrl: "https://example.test",
      requestedChannels: ["email", "link"],
      env: {
        WORKOS_API_KEY: "sk_test",
        WORKOS_CLIENT_ID: "client_test",
      },
    });

    expect(result.delivery.email).toEqual({
      status: "failed",
      reason: "notification_delivery_failed",
    });
    expect(JSON.stringify(result.delivery.email)).not.toContain("api key rejected");
    expect(result.invitation_token).toBe("token-123");
    expect(result.delivery.link).toMatchObject({ status: "available" });
  });

  it("redacts raw provider failures for email and SMS delivery attempts", async () => {
    const sendEmail = vi.fn(async () => {
      throw new Error("smtp socket hang up");
    });
    const sendSms = vi.fn(async () => {
      throw new Error("twilio auth token rejected");
    });

    const result = await deliverPaceInvitation(invitation, {
      appBaseUrl: "https://example.test",
      requestedChannels: ["email", "sms"],
      env: {
        SMTP_HOST: "smtp.example.test",
        SMTP_FROM: "LiveSafe <noreply@example.test>",
        TWILIO_ACCOUNT_SID: "sid",
        TWILIO_AUTH_TOKEN: "secret",
        TWILIO_FROM_NUMBER: "+15557654321",
      },
      transports: {
        sendEmail,
        sendSms,
      },
    });

    expect(result.delivery.email).toEqual({
      status: "failed",
      reason: "notification_delivery_failed",
    });
    expect(result.delivery.sms).toEqual({
      status: "failed",
      reason: "notification_delivery_failed",
    });
  });

  it("normalizes channel requests and keeps link available by default", () => {
    expect(normalizeDeliveryChannels(["email", "sms", "link", "email"])).toEqual([
      "email",
      "sms",
      "link",
    ]);
    expect(normalizeDeliveryChannels(undefined)).toEqual(["link"]);
  });
});
