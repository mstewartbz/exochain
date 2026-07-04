"use strict";

function buildAdminResponderResponse(responder = {}) {
  return {
    id: responder.id,
    email: responder.email || null,
    role: responder.role || null,
    certification: responder.certification || null,
    is_military: Boolean(responder.is_military),
    is_active: Boolean(responder.is_active),
    created_at: responder.created_at || null,
  };
}

function buildAdminResponderListResponse(rows = []) {
  const responders = rows.map(buildAdminResponderResponse);

  return {
    responders,
    total: responders.length,
    active: responders.filter((responder) => responder.is_active).length,
  };
}

function buildAdminAgencyResponderListResponse({
  agency = {},
  responders = [],
} = {}) {
  return {
    agency: {
      id: agency.id,
      name: agency.name || null,
    },
    ...buildAdminResponderListResponse(responders),
  };
}

function buildAdminResponderToggleResponse({
  is_active = false,
  responder = {},
} = {}) {
  return {
    message: `Responder ${is_active ? "activated" : "deactivated"} successfully`,
    responder: buildAdminResponderResponse(responder),
  };
}

module.exports = {
  buildAdminAgencyResponderListResponse,
  buildAdminResponderListResponse,
  buildAdminResponderResponse,
  buildAdminResponderToggleResponse,
};
