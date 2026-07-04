import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("subscriber scan history route redaction wiring", () => {
  it("routes subscriber history and detail reads through bounded scan helpers", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );
    const historyRouteBlock = scanRoute.slice(
      scanRoute.indexOf("router.get('/history/:subscriberDid'"),
      scanRoute.indexOf("// GET /api/scan/detail/:scanId"),
    );
    const detailRouteBlock = scanRoute.slice(
      scanRoute.indexOf("router.get('/detail/:scanId'"),
      scanRoute.indexOf("// GET /api/scan/agency - Get all scans for agency admin"),
    );

    expect(historyRouteBlock).toContain(
      "res.json(result.rows.map(buildSubscriberScanHistoryEntry));",
    );
    expect(detailRouteBlock).toContain(
      "res.json(buildSubscriberScanDetailResponse(result.rows[0]));",
    );
    expect(historyRouteBlock).not.toContain("res.json(result.rows);");
    expect(detailRouteBlock).not.toContain("res.json(result.rows[0]);");
  });
});
