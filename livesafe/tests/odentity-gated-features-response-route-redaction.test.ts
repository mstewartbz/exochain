import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity gated-features response route redaction wiring", () => {
  it("routes gated-features responses through the bounded helper and avoids top-level subscriber bindings", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );
    const meGatedStart = odentityRoute.indexOf("router.get('/me/gated-features'");
    const scopedScoreStart = odentityRoute.indexOf("router.get('/:subscriberId/score'");
    const scopedGatedStart = odentityRoute.indexOf("router.get('/:subscriberId/gated-features'");
    const exportStart = odentityRoute.indexOf("router.get('/me/export-vc'");
    const meGatedBlock = odentityRoute.slice(meGatedStart, scopedScoreStart);
    const scopedGatedBlock = odentityRoute.slice(scopedGatedStart, exportStart);

    expect(odentityRoute).toContain("buildPublicOdentityGatedFeaturesResponse({");
    expect(meGatedBlock).not.toContain("subscriber_id:");
    expect(scopedGatedBlock).not.toContain("subscriber_id:");
  });
});
