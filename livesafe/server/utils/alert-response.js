"use strict";

function toTypeLabel(notificationType) {
  return String(notificationType || "")
    .replace(/_/g, " ")
    .replace(/\b\w/g, (char) => char.toUpperCase());
}

function parseBody(body) {
  if (!body || typeof body !== "string") {
    return body;
  }

  try {
    return JSON.parse(body);
  } catch (_) {
    return body;
  }
}

function sanitizeAlertDetails(notificationType, rawDetails) {
  if (!rawDetails || typeof rawDetails !== "object" || Array.isArray(rawDetails)) {
    return rawDetails;
  }

  const allowlists = {
    pace_alert: ["message", "responding_agency", "trustee_role", "timestamp"],
    card_scan: ["scan_time", "responder_agency"],
    trustee_response: ["response_status", "response_message", "responded_at"],
  };

  const allowedKeys =
    allowlists[notificationType] ||
    ["message", "status", "reason", "timestamp", "response_status", "response_message", "responded_at"];

  const sanitized = {};
  for (const key of allowedKeys) {
    if (Object.prototype.hasOwnProperty.call(rawDetails, key)) {
      sanitized[key] = rawDetails[key];
    }
  }

  return sanitized;
}

function buildAlertNotificationResponse(row) {
  const parsedDetails = parseBody(row.body);
  const details = sanitizeAlertDetails(row.notification_type, parsedDetails);

  return {
    id: row.id,
    alert_type: row.notification_type,
    type_label: toTypeLabel(row.notification_type),
    title: row.title,
    time: row.sent_at,
    channel: row.channel || null,
    status: row.status || "sent",
    response_status: row.response || (row.read ? "acknowledged" : "sent"),
    response: row.response || null,
    responded_at: row.responded_at || null,
    response_message: row.response_message || null,
    details,
    read: Boolean(row.read),
    ...(row.trustee_role ? { trustee_role: row.trustee_role } : {}),
  };
}

function buildAlertNotificationListResponse(rows, options = {}) {
  const key = options.key || "notifications";
  const notifications = rows.map(buildAlertNotificationResponse);

  return {
    [key]: notifications,
    total: notifications.length,
    unread: notifications.filter((notification) => !notification.read).length,
  };
}

function buildPaceAlertHistoryResponse({
  alerts = [],
  trusteeCount = 0,
} = {}) {
  return {
    ...buildAlertNotificationListResponse(alerts, { key: "alerts" }),
    trustee_count: trusteeCount,
  };
}

function buildAlertDispatchResponse({
  trusteesNotified,
  channelsPerTrustee,
  totalNotifications,
  notificationsByTrustee,
}) {
  return {
    status: "dispatched",
    trustees_notified: trusteesNotified,
    channels_per_trustee: channelsPerTrustee,
    total_notifications: totalNotifications,
    alerts_sent: trusteesNotified,
    notifications_by_trustee: notificationsByTrustee.map((trustee) => ({
      trustee_role: trustee.trustee_role,
      notifications_sent: trustee.notifications_sent,
      channels: trustee.channels,
    })),
  };
}

function buildAlertResponseAcknowledgement({
  notification = {},
} = {}) {
  return {
    response: notification.response || null,
    responded_at: notification.responded_at || null,
    response_message: notification.response_message || null,
    message: notification.response
      ? `Response "${notification.response}" recorded successfully`
      : "Response recorded successfully",
  };
}

function buildSubscriberAlertEventResponse(row) {
  const parsedDetails = parseBody(row.body);
  const details = sanitizeAlertDetails(row.notification_type, parsedDetails);

  return {
    id: row.id,
    alert_type: row.notification_type,
    type_label: toTypeLabel(row.notification_type),
    title: row.title,
    time: row.sent_at,
    status: row.status || "sent",
    response_status: row.read ? "acknowledged" : "sent",
    trustees_alerted: row.trustees_alerted || 0,
    total_notifications: row.total_notifications || 0,
    details,
    read: Boolean(row.read),
  };
}

function buildSubscriberAlertEventsResponse({
  subscriberDid,
  subscriberName,
  events,
}) {
  const publicEvents = events.map(buildSubscriberAlertEventResponse);

  return {
    events: publicEvents,
    total: publicEvents.length,
    subscriber_did: subscriberDid,
    subscriber_name: subscriberName,
  };
}

module.exports = {
  buildAlertDispatchResponse,
  buildAlertNotificationListResponse,
  buildAlertNotificationResponse,
  buildPaceAlertHistoryResponse,
  buildAlertResponseAcknowledgement,
  buildSubscriberAlertEventResponse,
  buildSubscriberAlertEventsResponse,
  sanitizeAlertDetails,
};
