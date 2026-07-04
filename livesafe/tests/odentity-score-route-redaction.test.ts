import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity score route redaction wiring", () => {
  it("routes score responses through the bounded helper and keeps the legacy score route ownership-scoped", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );
    const meScoreStart = odentityRoute.indexOf("router.get('/me/score'");
    const meScoreEnd = odentityRoute.indexOf("router.get('/me/gated-features'");
    const scopedScoreStart = odentityRoute.indexOf("router.get('/:subscriberId/score'");
    const scopedScoreEnd = odentityRoute.indexOf("router.get('/me/claims'");
    const meScoreBlock = odentityRoute.slice(meScoreStart, meScoreEnd);
    const scopedScoreBlock = odentityRoute.slice(scopedScoreStart, scopedScoreEnd);

    expect(odentityRoute).toContain("buildPublicOdentityScoreResponse({");
    expect(odentityRoute).toContain("router.get('/me/score', authMiddleware");
    expect(odentityRoute).toContain("router.get('/:subscriberId/score', authMiddleware");
    expect(odentityRoute).toContain(
      "return res.status(403).json({ error: 'Access denied: you can only view your own 0dentity score' });",
    );
    expect(meScoreBlock).not.toContain("subscriber_id:");
    expect(scopedScoreBlock).not.toContain("subscriber_id:");
  });
});
