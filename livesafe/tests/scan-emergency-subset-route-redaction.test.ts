import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("scan responder emergency-subset route redaction wiring", () => {
  it("routes responder emergency-subset responses through the bounded helper", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );
    const routeStart = scanRoute.indexOf(
      "// GET /api/scan/data/:subscriberDid",
    );
    const routeEnd = scanRoute.indexOf(
      "// Auth middleware for subscriber JWT",
    );
    const routeBlock = scanRoute.slice(routeStart, routeEnd);

    expect(routeBlock).toContain("buildPublicResponderEmergencySubsetResponse({");
    expect(routeBlock).not.toContain("res.json({");
    expect(routeBlock).not.toContain("access_type: 'emergency_subset'");
    expect(routeBlock).not.toContain("insurance_visible_to_er: insuranceCredentials.length > 0");
  });
});
