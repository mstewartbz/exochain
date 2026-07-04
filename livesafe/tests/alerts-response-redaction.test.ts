import { describe, expect, it } from "vitest";

const {
  buildAlertDispatchResponse,
  buildAlertNotificationListResponse,
  buildPaceAlertHistoryResponse,
  buildAlertResponseAcknowledgement,
  buildSubscriberAlertEventsResponse,
} = require("../server/utils/alert-response.js");

describe("alert response redaction", () => {
  it("builds a bounded alert-notification list without routing fields or raw alert details", () => {
    const response = buildAlertNotificationListResponse([
      {
        id: 42,
        recipient_did: "did:exo:trustee:private",
        recipient_type: "trustee",
        channel: "sms",
        notification_type: "pace_alert",
        title: "PACE Alert: Alex Rivera",
        body: JSON.stringify({
          message: "Emergency card scanned",
          subscriber_did: "did:exo:subscriber:private",
          scan_location: "123 Main St",
          responding_agency: "Medic One",
          trustee_role: "Primary",
          scan_id: 9001,
          timestamp: "2026-06-06T05:00:00.000Z",
        }),
        status: "sent",
        read: false,
        sent_at: "2026-06-06T05:00:00.000Z",
        response: null,
        responded_at: null,
        response_message: null,
      },
    ]);

    expect(response).toEqual({
      notifications: [
        {
          id: 42,
          alert_type: "pace_alert",
          type_label: "Pace Alert",
          title: "PACE Alert: Alex Rivera",
          time: "2026-06-06T05:00:00.000Z",
          channel: "sms",
          status: "sent",
          response_status: "sent",
          response: null,
          responded_at: null,
          response_message: null,
          details: {
            message: "Emergency card scanned",
            responding_agency: "Medic One",
            trustee_role: "Primary",
            timestamp: "2026-06-06T05:00:00.000Z",
          },
          read: false,
        },
      ],
      total: 1,
      unread: 1,
    });
    expect(JSON.stringify(response)).not.toContain("recipient_did");
    expect(JSON.stringify(response)).not.toContain("subscriber_did");
    expect(JSON.stringify(response)).not.toContain("scan_location");
    expect(JSON.stringify(response)).not.toContain("scan_id");
  });

  it("builds a bounded dispatch response without trustee identity or raw notification rows", () => {
    const response = buildAlertDispatchResponse({
      trusteesNotified: 1,
      channelsPerTrustee: 3,
      totalNotifications: 3,
      notificationsByTrustee: [
        {
          trustee_id: 77,
          trustee_email: "trustee@example.com",
          trustee_role: "Primary",
          recipient_did: "did:exo:trustee:private",
          notifications_sent: 3,
          channels: ["sms", "push", "email"],
          notifications: [
            { id: 101, recipient_did: "did:exo:trustee:private" },
          ],
        },
      ],
    });

    expect(response).toEqual({
      status: "dispatched",
      trustees_notified: 1,
      channels_per_trustee: 3,
      total_notifications: 3,
      alerts_sent: 1,
      notifications_by_trustee: [
        {
          trustee_role: "Primary",
          notifications_sent: 3,
          channels: ["sms", "push", "email"],
        },
      ],
    });
    expect(JSON.stringify(response)).not.toContain("trustee@example.com");
    expect(JSON.stringify(response)).not.toContain("recipient_did");
    expect(JSON.stringify(response)).not.toContain("\"id\":101");
  });

  it("builds bounded subscriber alert events without raw notification-body identifiers or locations", () => {
    const response = buildSubscriberAlertEventsResponse({
      subscriberDid: "did:exo:subscriber:self",
      subscriberName: "Alex Rivera",
      events: [
        {
          id: 51,
          notification_type: "card_scan",
          title: "Emergency card scanned",
          sent_at: "2026-06-06T06:00:00.000Z",
          status: "sent",
          read: false,
          body: JSON.stringify({
            scan_time: "2026-06-06T05:59:45.000Z",
            responder_agency: "Medic One",
            subscriber_did: "did:exo:subscriber:self",
            subscriber_name: "Alex Rivera",
            scan_location: "123 Main St",
            scan_id: 9001,
          }),
          trustees_alerted: 2,
          total_notifications: 6,
        },
        {
          id: 52,
          notification_type: "trustee_response",
          title: "Trustee responded to PACE alert",
          sent_at: "2026-06-06T06:05:00.000Z",
          status: "sent",
          read: true,
          body: JSON.stringify({
            trustee_did: "did:exo:trustee:private",
            response_status: "available",
            response_message: "I can help",
            scan_id: 9001,
            responded_at: "2026-06-06T06:04:30.000Z",
          }),
          trustees_alerted: 0,
          total_notifications: 0,
        },
      ],
    });

    expect(response).toEqual({
      events: [
        {
          id: 51,
          alert_type: "card_scan",
          type_label: "Card Scan",
          title: "Emergency card scanned",
          time: "2026-06-06T06:00:00.000Z",
          status: "sent",
          response_status: "sent",
          trustees_alerted: 2,
          total_notifications: 6,
          details: {
            scan_time: "2026-06-06T05:59:45.000Z",
            responder_agency: "Medic One",
          },
          read: false,
        },
        {
          id: 52,
          alert_type: "trustee_response",
          type_label: "Trustee Response",
          title: "Trustee responded to PACE alert",
          time: "2026-06-06T06:05:00.000Z",
          status: "sent",
          response_status: "acknowledged",
          trustees_alerted: 0,
          total_notifications: 0,
          details: {
            response_status: "available",
            response_message: "I can help",
            responded_at: "2026-06-06T06:04:30.000Z",
          },
          read: true,
        },
      ],
      total: 2,
      subscriber_did: "did:exo:subscriber:self",
      subscriber_name: "Alex Rivera",
    });
    expect(JSON.stringify(response.events)).not.toContain("subscriber_did");
    expect(JSON.stringify(response.events)).not.toContain("scan_location");
    expect(JSON.stringify(response.events)).not.toContain("scan_id");
    expect(JSON.stringify(response.events)).not.toContain("trustee_did");
  });

  it("builds bounded pace-alert history without raw trustee rows or extra route-level wrappers", () => {
    const response = buildPaceAlertHistoryResponse({
      alerts: [
        {
          id: 42,
          recipient_did: "did:exo:trustee:private",
          recipient_type: "trustee",
          channel: "sms",
          notification_type: "pace_alert",
          title: "PACE Alert: Alex Rivera",
          body: JSON.stringify({
            message: "Emergency card scanned",
            subscriber_did: "did:exo:subscriber:private",
            scan_location: "123 Main St",
            responding_agency: "Medic One",
            trustee_role: "Primary",
            scan_id: 9001,
            timestamp: "2026-06-06T05:00:00.000Z",
          }),
          status: "sent",
          read: false,
          sent_at: "2026-06-06T05:00:00.000Z",
          response: null,
          responded_at: null,
          response_message: null,
          trustee_role: "Primary",
        },
      ],
      trusteeCount: 1,
    });

    expect(response).toEqual({
      alerts: [
        {
          id: 42,
          alert_type: "pace_alert",
          type_label: "Pace Alert",
          title: "PACE Alert: Alex Rivera",
          time: "2026-06-06T05:00:00.000Z",
          channel: "sms",
          status: "sent",
          response_status: "sent",
          response: null,
          responded_at: null,
          response_message: null,
          details: {
            message: "Emergency card scanned",
            responding_agency: "Medic One",
            trustee_role: "Primary",
            timestamp: "2026-06-06T05:00:00.000Z",
          },
          read: false,
          trustee_role: "Primary",
        },
      ],
      total: 1,
      unread: 1,
      trustee_count: 1,
    });
    expect(JSON.stringify(response)).not.toContain("recipient_did");
    expect(JSON.stringify(response)).not.toContain("subscriber_did");
    expect(JSON.stringify(response)).not.toContain("scan_location");
    expect(JSON.stringify(response)).not.toContain("scan_id");
  });

  it("builds a bounded trustee-response acknowledgement without raw notification ids or trustee bindings", () => {
    const response = buildAlertResponseAcknowledgement({
      notification: {
        id: 77,
        recipient_did: "did:exo:trustee:private",
        response: "available",
        responded_at: "2026-06-07T16:20:00.000Z",
        response_message: "I can help",
      },
    });

    expect(response).toEqual({
      response: "available",
      responded_at: "2026-06-07T16:20:00.000Z",
      response_message: "I can help",
      message: 'Response "available" recorded successfully',
    });
    expect(JSON.stringify(response)).not.toContain("notification_id");
    expect(JSON.stringify(response)).not.toContain("recipient_did");
    expect(JSON.stringify(response)).not.toContain("\"id\":77");
  });
});
