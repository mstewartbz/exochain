"use strict";

function parseDetails(details) {
  if (!details) {
    return null;
  }

  if (typeof details === "string") {
    try {
      return JSON.parse(details);
    } catch (_) {
      return null;
    }
  }

  if (typeof details === "object" && !Array.isArray(details)) {
    return details;
  }

  return null;
}

function sanitizeAuditDetails(eventType, rawDetails) {
  const parsed = parseDetails(rawDetails);
  if (!parsed) {
    return null;
  }

  const allowlists = {
    record_deleted: ["record_title", "record_type", "note"],
  };

  const allowedKeys = allowlists[eventType];
  if (!allowedKeys) {
    return null;
  }

  const sanitized = {};
  for (const key of allowedKeys) {
    if (Object.prototype.hasOwnProperty.call(parsed, key)) {
      sanitized[key] = parsed[key];
    }
  }

  return Object.keys(sanitized).length > 0 ? sanitized : null;
}

function buildAuditEventResponse(row) {
  return {
    id: row.id,
    actor_did: row.actor_did || null,
    event_type: row.event_type,
    scope: row.scope || null,
    details: sanitizeAuditDetails(row.event_type, row.details),
    receipt_hash: row.receipt_hash || null,
    created_at: row.created_at || null,
  };
}

function buildAuditTrailResponse(rows) {
  return rows.map(buildAuditEventResponse);
}

module.exports = {
  buildAuditEventResponse,
  buildAuditTrailResponse,
  sanitizeAuditDetails,
};
