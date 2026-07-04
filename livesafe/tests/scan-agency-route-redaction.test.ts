import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("scan agency route redaction wiring", () => {
  it("routes agency-admin scan lists through the bounded summary helper", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );

    expect(scanRoute).toContain("res.json(result.rows.map(buildAgencyScanSummary));");
    expect(scanRoute).not.toContain("sub.email as subscriber_email");
    expect(scanRoute).not.toContain("SELECT s.*, r.email as responder_email, r.did as responder_did, r.role as responder_role,");
  });

  it("routes agency-admin responder filters through a bounded responder helper", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );

    expect(scanRoute).toContain("res.json(result.rows.map(buildAgencyResponderSummary));");
    expect(scanRoute).not.toContain("SELECT id, did, email, role, certification FROM responders");
  });

  it("routes scan follow-up flag acknowledgements through a bounded helper", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );

    expect(scanRoute).toContain("buildScanFollowupMutationResponse({");
    expect(scanRoute).not.toContain("return res.json({\n        id: scan.id,");
    expect(scanRoute).not.toContain("res.json({\n      id: result.rows[0].id,");
  });
});
