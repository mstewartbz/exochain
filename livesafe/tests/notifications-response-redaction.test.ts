const {
  buildNotificationResponse,
  buildNotificationListResponse,
  buildNotificationMutationResponse,
  buildNotificationUnreadCountResponse,
} = require("../server/utils/notification-response.js");

describe("notification response redaction", () => {
  const notificationRow = {
    id: 42,
    recipient_did: "did:exo:subscriber:private",
    recipient_type: "subscriber",
    channel: "app",
    notification_type: "pace_alert",
    title: "PACE alert",
    body: JSON.stringify({
      subscriber_name: "Subscriber Example",
      trustee_role: "Primary",
    }),
    status: "sent",
    read: false,
    sent_at: "2026-06-06T04:25:57.000Z",
    created_at: "2026-06-06T04:25:57.000Z",
  };

  it("builds a bounded notification payload without recipient routing fields", () => {
    expect(buildNotificationResponse(notificationRow)).toEqual({
      id: 42,
      notification_type: "pace_alert",
      title: "PACE alert",
      body: JSON.stringify({
        subscriber_name: "Subscriber Example",
        trustee_role: "Primary",
      }),
      status: "sent",
      read: false,
      sent_at: "2026-06-06T04:25:57.000Z",
    });
  });

  it("builds a bounded notification list response without echoing recipient routing fields", () => {
    expect(buildNotificationListResponse([notificationRow])).toEqual({
      notifications: [
        {
          id: 42,
          notification_type: "pace_alert",
          title: "PACE alert",
          body: JSON.stringify({
            subscriber_name: "Subscriber Example",
            trustee_role: "Primary",
          }),
          status: "sent",
          read: false,
          sent_at: "2026-06-06T04:25:57.000Z",
        },
      ],
      total: 1,
      unread_count: 1,
    });
  });

  it("builds bounded notification mutation acknowledgements without raw notification ids", () => {
    expect(
      buildNotificationMutationResponse({
        message: "Notification dismissed",
      }),
    ).toEqual({
      message: "Notification dismissed",
    });

    expect(
      buildNotificationMutationResponse({
        message: "Marked 3 notifications as read",
        markedCount: 3,
      }),
    ).toEqual({
      message: "Marked 3 notifications as read",
      marked_count: 3,
    });

    expect(
      buildNotificationMutationResponse({
        message: "Dismissed 2 read notifications",
        dismissedCount: 2,
      }),
    ).toEqual({
      message: "Dismissed 2 read notifications",
      dismissed_count: 2,
    });
  });

  it("builds a bounded unread-count payload", () => {
    expect(buildNotificationUnreadCountResponse(7)).toEqual({
      unread_count: 7,
    });
  });
});
