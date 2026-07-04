"use strict";

const { buildAuditTrailResponse } = require("./audit-response");

function buildAdminAuditTrailResponse(rows = [], pagination = {}) {
  return {
    records: buildAuditTrailResponse(rows),
    total: Number(pagination.total ?? rows.length),
    page: Number(pagination.page ?? 1),
    limit: Number(pagination.limit ?? rows.length),
  };
}

module.exports = {
  buildAdminAuditTrailResponse,
};
