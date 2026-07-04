const fs = require("node:fs");
const path = require("node:path");

describe("notification route redaction wiring", () => {
  it("routes notification responses through bounded helpers instead of wildcard rows", () => {
    const notificationRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/notifications.js"),
      "utf8",
    );

    expect(notificationRoute).toContain("buildNotificationListResponse(result.rows)");
    expect(notificationRoute).toContain("buildNotificationResponse(result.rows[0])");
    expect(notificationRoute).toContain("buildNotificationMutationResponse({");
    expect(notificationRoute).toContain("buildNotificationUnreadCountResponse(");
    expect(notificationRoute).not.toContain("notifications: result.rows");
    expect(notificationRoute).not.toContain("res.status(201).json(result.rows[0]);");
    expect(notificationRoute).not.toContain("res.json({ ...result.rows[0], message: 'Notification marked as read' });");
    expect(notificationRoute).not.toContain("res.json({ message: 'Notification dismissed', id: parseInt(id) });");
    expect(notificationRoute).not.toContain("res.json({\n      marked_count: result.rows.length,");
    expect(notificationRoute).not.toContain("res.json({\n      message: `Dismissed ${result.rows.length} read notifications`,");
    expect(notificationRoute).not.toContain("res.json({ unread_count: parseInt(result.rows[0].count, 10) });");
    expect(notificationRoute).not.toContain("RETURNING *");
    expect(notificationRoute).not.toContain("SELECT id, recipient_did, notification_type, title, body, status, read, sent_at");
  });
});
