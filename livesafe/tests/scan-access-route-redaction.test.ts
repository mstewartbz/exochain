import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("scan token-access route redaction wiring", () => {
  it("routes token access responses through bounded helpers", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );
    const accessRouteStart = scanRoute.indexOf(
      "// GET /api/scan/access/:accessToken",
    );
    const accessRouteEnd = scanRoute.indexOf(
      "// GET /api/scan/data/:subscriberDid",
    );
    const accessRouteBlock = scanRoute.slice(accessRouteStart, accessRouteEnd);

    expect(accessRouteBlock).toContain("buildExpiredScanAccessResponse({");
    expect(accessRouteBlock).toContain("buildPublicScanAccessResponse({");
    expect(accessRouteBlock).not.toContain("access_token: accessToken");
    expect(accessRouteBlock).not.toContain("scan_id: scan.id");
  });
});
