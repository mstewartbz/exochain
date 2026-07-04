"use strict";

function buildAdminSubscriberResponse(subscriber = {}) {
  return {
    id: subscriber.id,
    email: subscriber.email || null,
    first_name: subscriber.first_name || null,
    last_name: subscriber.last_name || null,
    role: subscriber.role || null,
    email_verified: Boolean(subscriber.email_verified),
    created_at: subscriber.created_at || null,
    updated_at: subscriber.updated_at || null,
  };
}

function buildAdminSubscriberListResponse(rows = [], pagination = {}) {
  return {
    subscribers: rows.map(buildAdminSubscriberResponse),
    total: pagination.total ?? rows.length,
    page: pagination.page ?? 1,
    limit: pagination.limit ?? rows.length,
  };
}

module.exports = {
  buildAdminSubscriberListResponse,
  buildAdminSubscriberResponse,
};
