import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("alert route redaction wiring", () => {
  it("routes alert responses through bounded helpers instead of raw notification rows", () => {
    const alertRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/alerts.js"),
      "utf8",
    );

    expect(alertRoute).toContain("buildAlertDispatchResponse({");
    expect(alertRoute).toContain("buildAlertNotificationListResponse(");
    expect(alertRoute).toContain("buildPaceAlertHistoryResponse({");
    expect(alertRoute).toContain("buildAlertResponseAcknowledgement({");
    expect(alertRoute).toContain("buildSubscriberAlertEventsResponse({");
    expect(alertRoute).not.toContain("res.json(result.rows);");
    expect(alertRoute).not.toContain("notifications_by_trustee: notificationsByTrustee");
    expect(alertRoute).not.toContain("notifications: allNotifications");
    expect(alertRoute).not.toContain("notification_id: updated.id");
    expect(alertRoute).not.toContain("trustee_email: trustee.email");
    expect(alertRoute).not.toContain("recipient_did: recipientId");
    expect(alertRoute).not.toContain("details: body");
    expect(alertRoute).not.toContain("...buildAlertNotificationListResponse(allAlerts, { key: 'alerts' })");
    expect(alertRoute).not.toContain("trustee_count: trustees.rows.length");
    expect(alertRoute).not.toContain("subscriber_name: [subscriber.first_name, subscriber.last_name].filter(Boolean).join(' ')");
  });
});
