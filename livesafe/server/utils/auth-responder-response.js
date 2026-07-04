"use strict";

function buildPublicResponderAuthResponse(responder = {}) {
  return {
    id: responder.id,
    did: responder.did || null,
    email: responder.email || null,
    agency_name: responder.agency_name || null,
    agency_type: responder.agency_type || null,
    role: responder.role || null,
    certification: responder.certification || null,
    is_military: Boolean(responder.is_military),
    user_type: "responder",
    tier: "free",
  };
}

function buildPublicResponderAuthSessionResponse({ user = {}, token = null } = {}) {
  return {
    user: buildPublicResponderAuthResponse(user),
    token,
  };
}

function buildPublicAgencyRegistrationResponse({
  agency = {},
  admin = {},
} = {}) {
  return {
    agency: buildPublicAgencyDirectoryEntry(agency),
    user: buildPublicResponderAuthResponse(admin),
  };
}

function buildPublicAgencyRegistrationSessionResponse({
  agency = {},
  admin = {},
  token = null,
} = {}) {
  return {
    ...buildPublicAgencyRegistrationResponse({
      agency,
      admin,
    }),
    token,
  };
}

function buildPublicAgencyDirectoryEntry(agency = {}) {
  return {
    id: agency.id,
    name: agency.name || null,
    type: agency.type || null,
    verified: Boolean(agency.verified),
  };
}

module.exports = {
  buildPublicResponderAuthResponse,
  buildPublicResponderAuthSessionResponse,
  buildPublicAgencyDirectoryEntry,
  buildPublicAgencyRegistrationResponse,
  buildPublicAgencyRegistrationSessionResponse,
};
