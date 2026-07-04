"use strict";

function buildPublicProviderAuthResponse(provider = {}) {
  return {
    id: provider.id,
    did: provider.did || null,
    email: provider.email || null,
    npi: provider.npi || null,
    facility: provider.facility || null,
    specialty: provider.specialty || null,
    verified: Boolean(provider.verified),
    npi_verified: Boolean(provider.npi_verified),
    provider_name: provider.provider_name || null,
    npi_taxonomy: provider.npi_taxonomy || null,
    user_type: "provider",
    tier: "free",
  };
}

function buildPublicProviderAuthSessionResponse({ user, token } = {}) {
  return {
    user: buildPublicProviderAuthResponse(user),
    token,
  };
}

function buildPublicProviderConsentResponse(consent = {}) {
  return {
    id: consent.id,
    subscriber_name: consent.subscriber_name || "Anonymous",
    scope: consent.scope || null,
    purpose: consent.purpose || null,
    created_at: consent.created_at || null,
    expires_at: consent.expires_at || null,
    revoked_at: consent.revoked_at || null,
  };
}

function buildPublicProviderAuthProfileResponse({
  provider,
  consents = [],
} = {}) {
  return {
    ...buildPublicProviderAuthResponse(provider),
    consents: consents.map(buildPublicProviderConsentResponse),
  };
}

function buildPublicProviderNpiLookupAddress(address = {}) {
  return {
    address_purpose: address.address_purpose || null,
    city: address.city || null,
    state: address.state || null,
    postal_code: address.postal_code || null,
  };
}

function buildPublicProviderNpiLookupResponse(lookup = {}) {
  return {
    valid: true,
    npi: lookup.npi || null,
    provider_name: lookup.provider_name || null,
    taxonomy: lookup.taxonomy_description || lookup.taxonomy || null,
    facility: lookup.facility || null,
    status: lookup.status || null,
    enumeration_type: lookup.enumeration_type || null,
    enumeration_date: lookup.enumeration_date || null,
    last_updated: lookup.last_updated || null,
    addresses: Array.isArray(lookup.addresses)
      ? lookup.addresses.map(buildPublicProviderNpiLookupAddress)
      : [],
  };
}

module.exports = {
  buildPublicProviderAuthResponse,
  buildPublicProviderAuthSessionResponse,
  buildPublicProviderConsentResponse,
  buildPublicProviderAuthProfileResponse,
  buildPublicProviderNpiLookupResponse,
};
