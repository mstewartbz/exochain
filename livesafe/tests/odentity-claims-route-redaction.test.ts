import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity claims route redaction wiring", () => {
  it("routes claim responses through the bounded helper and requires auth on the legacy claims route", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );

    expect(odentityRoute).toContain("buildPublicOdentityClaimListResponse(result.rows)");
    expect(odentityRoute).toContain("router.get('/me/claims', authMiddleware");
    expect(odentityRoute).toContain("router.get('/:subscriberId/claims', authMiddleware");
    expect(odentityRoute).toContain(
      "return res.status(403).json({ error: 'Access denied: you can only view your own 0dentity claims' });",
    );
    expect(odentityRoute).not.toContain("router.get('/:subscriberId/claims', async");
    expect(odentityRoute).not.toContain("res.json(result.rows);");
  });
});
