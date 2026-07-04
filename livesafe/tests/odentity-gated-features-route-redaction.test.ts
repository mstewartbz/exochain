import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity gated-features route ownership", () => {
  it("requires auth and same-subscriber ownership on the legacy gated-features route", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );

    expect(odentityRoute).toContain("router.get('/me/gated-features', authMiddleware");
    expect(odentityRoute).toContain("router.get('/:subscriberId/gated-features', authMiddleware");
    expect(odentityRoute).toContain(
      "return res.status(403).json({ error: 'Access denied: you can only view your own gated features' });",
    );
    expect(odentityRoute).not.toContain("router.get('/:subscriberId/gated-features', async");
  });
});
