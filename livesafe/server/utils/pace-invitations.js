const https = require("https");
const { getPaceRoleDetails, normalizePaceRole } = require("./pace-roles");

const VALID_DELIVERY_CHANNELS = Object.freeze(["email", "sms", "link"]);
const DELIVERY_FAILURE_REASON = "notification_delivery_failed";
const WORKOS_API_BASE_URL = "https://api.workos.com";
const DEFAULT_INVITATION_EXPIRES_IN_DAYS = 7;

function cleanBaseUrl(value) {
  const origin = String(value || "").trim().replace(/\/+$/, "");
  if (!origin) {
    return null;
  }
  return origin;
}

function firstHeaderValue(value) {
  if (!value) {
    return null;
  }
  return String(value).split(",")[0].trim();
}

function requestHeader(req, name) {
  if (!req || typeof req.get !== "function") {
    return null;
  }
  return firstHeaderValue(req.get(name));
}

function headerOrigin(value) {
  const headerValue = firstHeaderValue(value);
  if (!headerValue) {
    return null;
  }

  try {
    return new URL(headerValue).origin;
  } catch (_err) {
    return null;
  }
}

function isLocalhostHost(host) {
  const normalized = String(host || "")
    .toLowerCase()
    .replace(/^\[/, "")
    .replace(/\]$/, "")
    .split(":")[0];

  return normalized === "localhost" || normalized === "127.0.0.1" || normalized === "::1";
}

function resolveInvitationBaseUrl(options = {}) {
  const env = options.env || process.env;
  const configured =
    options.appBaseUrl ||
    env.LIVESAFE_PUBLIC_APP_URL ||
    env.PUBLIC_APP_URL ||
    env.CLIENT_URL ||
    env.FRONTEND_URL ||
    env.VITE_PUBLIC_APP_URL;

  if (configured) {
    return cleanBaseUrl(configured);
  }

  if (env.RAILWAY_PUBLIC_DOMAIN) {
    return cleanBaseUrl(`https://${env.RAILWAY_PUBLIC_DOMAIN}`);
  }

  const req = options.req;
  const forwardedProto = requestHeader(req, "x-forwarded-proto");
  const proto = forwardedProto || req?.protocol || "http";
  const forwardedHost = requestHeader(req, "x-forwarded-host");
  if (forwardedHost) {
    return cleanBaseUrl(`${proto}://${forwardedHost}`);
  }

  const host = requestHeader(req, "host");
  if (host) {
    const origin = headerOrigin(requestHeader(req, "origin") || requestHeader(req, "referer"));
    if (origin && isLocalhostHost(host) && isLocalhostHost(new URL(origin).host)) {
      return cleanBaseUrl(origin);
    }

    if (/^(localhost|127\.0\.0\.1):3001$/i.test(host)) {
      return cleanBaseUrl(`${proto}://${host.replace(/:3001$/i, ":3000")}`);
    }

    return cleanBaseUrl(`${proto}://${host}`);
  }

  return "http://localhost:3000";
}

function normalizeDeliveryChannels(channels) {
  const requested = Array.isArray(channels) && channels.length > 0 ? channels : ["link"];
  const normalized = [];

  for (const channel of requested) {
    const value = String(channel || "").trim().toLowerCase();
    if (VALID_DELIVERY_CHANNELS.includes(value) && !normalized.includes(value)) {
      normalized.push(value);
    }
  }

  return normalized.length > 0 ? normalized : ["link"];
}

function buildInvitationUrl({ baseUrl, token }) {
  const origin = String(baseUrl || "").trim().replace(/\/+$/, "");
  if (!origin) {
    throw new Error("Invitation URL requires an application base URL.");
  }
  if (!token) {
    throw new Error("Invitation URL requires an invitation token.");
  }

  return `${origin}/trustee/accept?token=${encodeURIComponent(token)}`;
}

function createPaceInvitationMessages(invitation, { invitationUrl }) {
  const role = getPaceRoleDetails(invitation.role);
  const subscriberName = invitation.subscriberName || "Someone";

  return {
    sms: `${subscriberName} named you as a LiveSafe P.A.C.E. contact. Accept or decline: ${invitationUrl}`,
    email: {
      subject: `${subscriberName} is asking you to accept a LiveSafe P.A.C.E. role`,
      text: [
        `${subscriberName} named you as a P.A.C.E. emergency contact.`,
        "",
        "This is not a marketing invite. This is a human trust request.",
        "",
        `${subscriberName} is asking whether you are willing to be one of four trusted people who may be notified if ${subscriberName}'s LiveSafe emergency card is scanned or if ${subscriberName} cannot speak for themself.`,
        "",
        `Your proposed role: ${role.name}.`,
        `${role.name} means ${role.description}`,
        "",
        `Accepting this role does not give you ${subscriberName}'s full medical records.`,
        "You may only see information the subscriber explicitly shares with you or makes available for emergency purposes.",
        "",
        `You can accept, decline, or ask ${subscriberName} to choose someone else.`,
        "You can also revoke later if your availability changes.",
        "",
        "When the full P.A.C.E. circle is complete, LiveSafe may grant the subscriber a readiness credit. Your decision should be based only on whether you are willing to serve in this role.",
        "",
        `Accept or decline: ${invitationUrl}`,
      ].join("\n"),
    },
  };
}

function emailTransportConfigured(env) {
  return Boolean(
    env.WORKOS_API_KEY ||
      env.SMTP_HOST ||
      env.SENDGRID_API_KEY ||
      env.MAILGUN_API_KEY ||
      env.RESEND_API_KEY,
  );
}

function smsTransportConfigured(env) {
  return Boolean(env.TWILIO_ACCOUNT_SID && env.TWILIO_AUTH_TOKEN && env.TWILIO_FROM_NUMBER);
}

async function sendEmailWithConfiguredProvider(message, env) {
  if (env.WORKOS_API_KEY) {
    return sendWorkosInvitationEmail(message, env);
  }

  if (env.SENDGRID_API_KEY) {
    return sendSendgridEmail(message, env);
  }

  if (env.RESEND_API_KEY) {
    return sendResendEmail(message, env);
  }

  if (env.SMTP_HOST) {
    let nodemailer;
    try {
      nodemailer = require("nodemailer");
    } catch (_err) {
      throw new Error("email_transport_dependency_missing");
    }

    const transporter = nodemailer.createTransport({
      host: env.SMTP_HOST,
      port: Number(env.SMTP_PORT || 587),
      secure: String(env.SMTP_SECURE || "").toLowerCase() === "true",
      auth: env.SMTP_USER
        ? {
            user: env.SMTP_USER,
            pass: env.SMTP_PASS,
          }
        : undefined,
    });
    const response = await transporter.sendMail({
      from: env.SMTP_FROM,
      to: message.to,
      subject: message.subject,
      text: message.text,
    });
    return { providerMessageId: response.messageId || null };
  }

  throw new Error("email_transport_not_configured");
}

function optionalString(value) {
  const normalized = String(value || "").trim();
  return normalized || null;
}

function positiveIntegerOrDefault(value, fallback) {
  if (value === undefined || value === null || value === "") {
    return fallback;
  }

  const parsed = Number(value);
  if (!Number.isInteger(parsed) || parsed < 1) {
    return fallback;
  }

  return parsed;
}

function buildWorkosInvitationPayload(message, env) {
  const payload = {
    email: message.to,
    expires_in_days: positiveIntegerOrDefault(
      env.WORKOS_INVITATION_EXPIRES_IN_DAYS,
      DEFAULT_INVITATION_EXPIRES_IN_DAYS,
    ),
  };
  const organizationId = optionalString(
    env.WORKOS_INVITATION_ORGANIZATION_ID || env.WORKOS_ORGANIZATION_ID,
  );
  const roleSlug = optionalString(
    env.WORKOS_INVITATION_ROLE_SLUG || env.WORKOS_ROLE_SLUG,
  );
  const inviterUserId = optionalString(
    env.WORKOS_INVITATION_INVITER_USER_ID || env.WORKOS_INVITER_USER_ID,
  );
  const locale = optionalString(env.WORKOS_INVITATION_LOCALE);

  if (organizationId) {
    payload.organization_id = organizationId;
  }
  if (roleSlug) {
    payload.role_slug = roleSlug;
  }
  if (inviterUserId) {
    payload.inviter_user_id = inviterUserId;
  }
  if (locale) {
    payload.locale = locale;
  }

  return payload;
}

async function sendWorkosInvitationEmail(message, env) {
  const apiBaseUrl = cleanBaseUrl(env.WORKOS_API_BASE_URL || WORKOS_API_BASE_URL);
  const response = await fetch(`${apiBaseUrl}/user_management/invitations`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.WORKOS_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(buildWorkosInvitationPayload(message, env)),
  });

  const payload = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(`workos_${response.status}`);
  }

  return {
    providerMessageId: payload.id || null,
    providerInvitationToken: payload.token || null,
    providerAcceptInvitationUrl: payload.accept_invitation_url || null,
  };
}

async function sendSendgridEmail(message, env) {
  const response = await fetch("https://api.sendgrid.com/v3/mail/send", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.SENDGRID_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      personalizations: [{ to: [{ email: message.to }] }],
      from: { email: env.SMTP_FROM || env.SENDGRID_FROM },
      subject: message.subject,
      content: [{ type: "text/plain", value: message.text }],
    }),
  });

  if (!response.ok) {
    throw new Error(`sendgrid_${response.status}`);
  }

  return { providerMessageId: response.headers.get("x-message-id") || null };
}

async function sendResendEmail(message, env) {
  const response = await fetch("https://api.resend.com/emails", {
    method: "POST",
    headers: {
      Authorization: `Bearer ${env.RESEND_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      from: env.SMTP_FROM || env.RESEND_FROM,
      to: [message.to],
      subject: message.subject,
      text: message.text,
    }),
  });

  const payload = await response.json().catch(() => ({}));
  if (!response.ok) {
    throw new Error(`resend_${response.status}`);
  }

  return { providerMessageId: payload.id || null };
}

async function sendSmsWithConfiguredProvider(message, env) {
  if (!smsTransportConfigured(env)) {
    throw new Error("sms_transport_not_configured");
  }

  const body = new URLSearchParams({
    To: message.to,
    From: env.TWILIO_FROM_NUMBER,
    Body: message.body,
  }).toString();
  const auth = Buffer.from(`${env.TWILIO_ACCOUNT_SID}:${env.TWILIO_AUTH_TOKEN}`).toString("base64");

  return new Promise((resolve, reject) => {
    const req = https.request(
      {
        method: "POST",
        hostname: "api.twilio.com",
        path: `/2010-04-01/Accounts/${encodeURIComponent(env.TWILIO_ACCOUNT_SID)}/Messages.json`,
        headers: {
          Authorization: `Basic ${auth}`,
          "Content-Type": "application/x-www-form-urlencoded",
          "Content-Length": Buffer.byteLength(body),
        },
      },
      (res) => {
        let raw = "";
        res.on("data", (chunk) => {
          raw += chunk;
        });
        res.on("end", () => {
          const payload = raw ? JSON.parse(raw) : {};
          if (res.statusCode < 200 || res.statusCode >= 300) {
            reject(new Error(`twilio_${res.statusCode}`));
            return;
          }
          resolve({ providerMessageId: payload.sid || null });
        });
      },
    );
    req.on("error", reject);
    req.write(body);
    req.end();
  });
}

async function deliverPaceInvitation(invitation, options = {}) {
  const env = options.env || process.env;
  const appBaseUrl = resolveInvitationBaseUrl({
    appBaseUrl: options.appBaseUrl,
    env,
  });
  const channels = normalizeDeliveryChannels(options.requestedChannels);
  const canonicalRole = normalizePaceRole(invitation.role);
  let effectiveInvitationToken = invitation.invitationToken;
  let invitationUrl = buildInvitationUrl({
    baseUrl: appBaseUrl,
    token: effectiveInvitationToken,
  });
  let messages = createPaceInvitationMessages(
    { ...invitation, role: canonicalRole },
    { invitationUrl },
  );
  const delivery = {
    link: {
      status: "available",
      invitation_url: invitationUrl,
    },
  };
  const transports = options.transports || {};

  if (channels.includes("email")) {
    if (!invitation.email) {
      delivery.email = { status: "blocked", reason: "email_required" };
    } else if (!emailTransportConfigured(env)) {
      delivery.email = { status: "blocked", reason: "email_transport_not_configured" };
    } else {
      try {
        const sendEmail =
          transports.sendEmail ||
          ((message) => sendEmailWithConfiguredProvider(message, env));
        const response = await sendEmail({
          to: invitation.email,
          subject: messages.email.subject,
          text: messages.email.text,
        });
        if (response.providerInvitationToken) {
          effectiveInvitationToken = response.providerInvitationToken;
          invitationUrl = buildInvitationUrl({
            baseUrl: appBaseUrl,
            token: effectiveInvitationToken,
          });
          delivery.link.invitation_url = invitationUrl;
          messages = createPaceInvitationMessages(
            { ...invitation, role: canonicalRole, invitationToken: effectiveInvitationToken },
            { invitationUrl },
          );
        }
        delivery.email = {
          status: "sent",
          provider_message_id: response.providerMessageId || null,
        };
      } catch (err) {
        delivery.email = {
          status: "failed",
          reason: DELIVERY_FAILURE_REASON,
        };
      }
    }
  }

  if (channels.includes("sms")) {
    if (!invitation.phone) {
      delivery.sms = { status: "blocked", reason: "phone_required" };
    } else if (!smsTransportConfigured(env)) {
      delivery.sms = { status: "blocked", reason: "sms_transport_not_configured" };
    } else {
      try {
        const sendSms =
          transports.sendSms ||
          ((message) => sendSmsWithConfiguredProvider(message, env));
        const response = await sendSms({
          to: invitation.phone,
          body: messages.sms,
        });
        delivery.sms = {
          status: "sent",
          provider_message_id: response.providerMessageId || null,
        };
      } catch (err) {
        delivery.sms = {
          status: "failed",
          reason: DELIVERY_FAILURE_REASON,
        };
      }
    }
  }

  return {
    invitation_url: invitationUrl,
    invitation_token: effectiveInvitationToken,
    delivery,
    message_body_stored: false,
  };
}

module.exports = {
  buildInvitationUrl,
  createPaceInvitationMessages,
  deliverPaceInvitation,
  normalizeDeliveryChannels,
  resolveInvitationBaseUrl,
  sendWorkosInvitationEmail,
  sendEmailWithConfiguredProvider,
  sendSmsWithConfiguredProvider,
};
