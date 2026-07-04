"use strict";

function buildAdminAgencyResponse(agency = {}) {
  return {
    id: agency.id,
    name: agency.name || null,
    type: agency.type || null,
    is_active: Boolean(agency.is_active),
    created_at: agency.created_at || null,
    responder_count: Number(agency.responder_count ?? 0),
    active_responders: Number(agency.active_responders ?? 0),
  };
}

function buildAdminAgencyListResponse(rows = []) {
  const agencies = rows.map(buildAdminAgencyResponse);

  return {
    agencies,
    total: agencies.length,
  };
}

function buildAdminAgencyMutationResponse({
  agency = {},
  message = null,
  affected_responders = 0,
} = {}) {
  return {
    message,
    agency: buildAdminAgencyResponse(agency),
    affected_responders: Number(affected_responders ?? 0),
  };
}

module.exports = {
  buildAdminAgencyListResponse,
  buildAdminAgencyMutationResponse,
  buildAdminAgencyResponse,
};
