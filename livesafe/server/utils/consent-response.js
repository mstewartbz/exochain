"use strict";

function buildConsentStatus(row, now = new Date()) {
  const isRevoked = Boolean(row?.revoked_at);
  const isExpired =
    !isRevoked &&
    Boolean(row?.expires_at) &&
    new Date(row.expires_at).getTime() <= now.getTime();
  const isActive = !isRevoked && !isExpired;

  return {
    consent_status: isRevoked ? "revoked" : isExpired ? "expired" : "active",
    is_active: isActive,
    is_expired: isExpired,
    is_revoked: isRevoked,
  };
}

function buildConsentResponse(row, options = {}) {
  if (!row || typeof row !== "object") {
    return null;
  }

  const status = buildConsentStatus(row, options.now ?? new Date());

  return {
    id: row.id,
    scope: row.scope,
    purpose: row.purpose ?? null,
    granted_at: row.granted_at ?? null,
    expires_at: row.expires_at ?? null,
    revoked_at: row.revoked_at ?? null,
    provider_name: row.provider_name ?? null,
    provider_npi: row.provider_npi ?? row.npi ?? null,
    provider_facility: row.provider_facility ?? row.facility ?? null,
    provider_specialty: row.provider_specialty ?? row.specialty ?? null,
    ...status,
  };
}

function buildConsentProviderResponse(row) {
  if (!row || typeof row !== "object") {
    return null;
  }

  return {
    id: row.id,
    provider_name: row.provider_name ?? null,
    npi: row.provider_npi ?? row.npi ?? null,
    facility: row.provider_facility ?? row.facility ?? null,
    specialty: row.provider_specialty ?? row.specialty ?? null,
    npi_taxonomy: row.provider_taxonomy ?? row.npi_taxonomy ?? null,
    npi_verified: Boolean(row.npi_verified),
    created_at: row.created_at ?? null,
  };
}

function buildConsentProviderListResponse(rows) {
  return rows.map((row) => buildConsentProviderResponse(row));
}

function buildSubscriberAccessRequestResponse(row) {
  if (!row || typeof row !== "object") {
    return null;
  }

  return {
    id: row.id,
    requested_scope: row.requested_scope ?? null,
    purpose: row.purpose ?? null,
    status: row.status ?? null,
    requested_at: row.requested_at ?? null,
    responded_at: row.responded_at ?? null,
    consent_id: row.consent_id ?? null,
    provider_name: row.provider_name ?? null,
    npi: row.provider_npi ?? row.npi ?? null,
    facility: row.provider_facility ?? row.facility ?? null,
    specialty: row.provider_specialty ?? row.specialty ?? null,
  };
}

function buildSubscriberAccessRequestListResponse(rows) {
  return rows.map((row) => buildSubscriberAccessRequestResponse(row));
}

function buildProviderAccessRequestResponse(row) {
  if (!row || typeof row !== "object") {
    return null;
  }

  return {
    id: row.id,
    requested_scope: row.requested_scope ?? null,
    purpose: row.purpose ?? null,
    status: row.status ?? null,
    requested_at: row.requested_at ?? null,
    responded_at: row.responded_at ?? null,
    consent_id: row.consent_id ?? null,
    subscriber_name: row.subscriber_name ?? null,
  };
}

function buildProviderAccessRequestListResponse(rows) {
  return rows.map((row) => buildProviderAccessRequestResponse(row));
}

function buildProviderAccessRequestCreateAcknowledgement({
  request,
  message,
} = {}) {
  return {
    request: buildProviderAccessRequestResponse(request),
    message: message ?? null,
  };
}

function buildConsentListResponse(rows, options = {}) {
  return rows.map((row) => buildConsentResponse(row, options));
}

function buildConsentCollectionResponse(rows, options = {}) {
  const consents = buildConsentListResponse(rows, options);

  return {
    consents,
    active_consents: consents.filter((consent) => consent.is_active),
    expired_consents: consents.filter((consent) => consent.is_expired),
    revoked_consents: consents.filter((consent) => consent.is_revoked),
    total: consents.length,
    active_count: consents.filter((consent) => consent.is_active).length,
    expired_count: consents.filter((consent) => consent.is_expired).length,
    revoked_count: consents.filter((consent) => consent.is_revoked).length,
  };
}

function buildConsentGrantAcknowledgement({
  consent,
  auditReceipt = null,
  message,
  idempotent = false,
} = {}) {
  return {
    consent: buildConsentResponse(consent),
    audit_receipt: auditReceipt,
    message: message ?? null,
    idempotent: Boolean(idempotent),
  };
}

function buildConsentRevocationAcknowledgement({
  consent,
  message,
  alreadyRevoked = false,
} = {}) {
  return {
    consent: buildConsentResponse(consent),
    message: message ?? null,
    already_revoked: Boolean(alreadyRevoked),
  };
}

function buildConsentAccessCheckResponse(consent = null) {
  return {
    has_access: Boolean(consent),
    consent: buildConsentResponse(consent),
  };
}

function buildConsentExpiryCheckResponse({ notifiedCount = 0 } = {}) {
  return {
    checked: true,
    expired_consents_notified: notifiedCount,
  };
}

function buildSubscriberAccessRequestApprovalResponse({
  consent,
  request,
  auditReceipt = null,
  message,
} = {}) {
  return {
    consent: buildConsentResponse(consent),
    request: buildSubscriberAccessRequestResponse(request),
    audit_receipt: auditReceipt,
    message: message ?? null,
  };
}

function buildSubscriberAccessRequestDenialResponse(message = "Access request denied") {
  return {
    message,
    status: "denied",
  };
}

module.exports = {
  buildConsentResponse,
  buildConsentListResponse,
  buildConsentCollectionResponse,
  buildConsentGrantAcknowledgement,
  buildConsentRevocationAcknowledgement,
  buildConsentAccessCheckResponse,
  buildConsentExpiryCheckResponse,
  buildConsentProviderResponse,
  buildConsentProviderListResponse,
  buildSubscriberAccessRequestResponse,
  buildSubscriberAccessRequestListResponse,
  buildSubscriberAccessRequestApprovalResponse,
  buildSubscriberAccessRequestDenialResponse,
  buildProviderAccessRequestResponse,
  buildProviderAccessRequestCreateAcknowledgement,
  buildProviderAccessRequestListResponse,
};
