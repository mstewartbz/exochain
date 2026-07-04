"use strict";

function maskEmail(email) {
  if (typeof email !== "string") {
    return null;
  }

  const trimmed = email.trim().toLowerCase();
  const atIndex = trimmed.indexOf("@");

  if (atIndex <= 0 || atIndex === trimmed.length - 1) {
    return null;
  }

  const localPart = trimmed.slice(0, atIndex);
  const domainPart = trimmed.slice(atIndex + 1);

  return `${localPart[0]}***@${domainPart}`;
}

function maskPhone(phone) {
  if (typeof phone !== "string") {
    return null;
  }

  const digits = phone.replace(/\D/g, "");
  if (digits.length < 4) {
    return null;
  }

  return `***-***-${digits.slice(-4)}`;
}

function buildPublicEmailVerificationResponse({
  email,
  alreadyVerified = false,
} = {}) {
  return {
    message: alreadyVerified
      ? "Email already verified"
      : "Email verified successfully",
    verified: true,
    already_verified: Boolean(alreadyVerified),
    verification_target: maskEmail(email),
  };
}

function buildPublicPhoneVerificationRequestResponse({
  phone,
  expiresAt,
  devCode,
} = {}) {
  const response = {
    message: "Verification code sent",
    verification_target: maskPhone(phone),
    expires_at: expiresAt || null,
  };

  if (devCode) {
    response.dev_code = devCode;
  }

  return response;
}

function buildPublicPhoneVerificationConfirmResponse({
  phone,
  alreadyVerified = false,
  identityCorePointsAwarded = 0,
} = {}) {
  return {
    message: alreadyVerified
      ? "Phone already verified"
      : "Phone verified successfully",
    verified: true,
    already_verified: Boolean(alreadyVerified),
    verification_target: maskPhone(phone),
    identity_core_points_awarded: identityCorePointsAwarded,
  };
}

module.exports = {
  buildPublicEmailVerificationResponse,
  buildPublicPhoneVerificationRequestResponse,
  buildPublicPhoneVerificationConfirmResponse,
};
