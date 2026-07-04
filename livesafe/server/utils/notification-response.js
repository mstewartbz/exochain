"use strict";

function buildNotificationResponse(row) {
  return {
    id: row.id,
    notification_type: row.notification_type,
    title: row.title,
    body: row.body,
    status: row.status,
    read: Boolean(row.read),
    sent_at: row.sent_at,
  };
}

function buildNotificationListResponse(rows) {
  const notifications = rows.map(buildNotificationResponse);

  return {
    notifications,
    total: notifications.length,
    unread_count: notifications.filter((notification) => !notification.read).length,
  };
}

function buildNotificationMutationResponse({
  message,
  notification,
  markedCount,
  dismissedCount,
}) {
  const response = { message };

  if (notification) {
    response.notification = buildNotificationResponse(notification);
  }

  if (Number.isInteger(markedCount)) {
    response.marked_count = markedCount;
  }

  if (Number.isInteger(dismissedCount)) {
    response.dismissed_count = dismissedCount;
  }

  return response;
}

function buildNotificationUnreadCountResponse(unreadCount) {
  return {
    unread_count: unreadCount,
  };
}

module.exports = {
  buildNotificationListResponse,
  buildNotificationMutationResponse,
  buildNotificationResponse,
  buildNotificationUnreadCountResponse,
};
