import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity trust-event route redaction wiring", () => {
  it("routes trust-event acknowledgements through a bounded helper and requires authenticated actor-target authority", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );
    const eventsStart = odentityRoute.indexOf("router.post('/events/record'");
    const gatedFeaturesStart = odentityRoute.indexOf("router.get('/:subscriberId/gated-features'");
    const eventsBlock = odentityRoute.slice(eventsStart, gatedFeaturesStart);

    expect(odentityRoute).toContain(
      "buildPublicOdentityTrustEventResponse(result.rows[0])",
    );
    expect(odentityRoute).toContain("router.post('/events/record', authMiddleware");
    expect(eventsBlock).toContain(
      "return res.status(403).json({ error: 'Forbidden: you can only record 0dentity trust events for your own subscriber account' });",
    );
    expect(eventsBlock).toContain(
      "return res.status(403).json({ error: 'Forbidden: you can only target your own subscriber account when recording 0dentity trust events' });",
    );
    expect(eventsBlock).not.toContain("res.status(201).json(result.rows[0]);");
    expect(eventsBlock).not.toContain("message: 'Trust event recorded successfully'");
    expect(eventsBlock).not.toContain("event: result.rows[0]");
  });
});
