import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("subscriber scan history UI redaction", () => {
  it("renders bounded responder and location fields on the scan history page", () => {
    const historyPage = fs.readFileSync(
      path.join(process.cwd(), "client/src/pages/ScanHistory.jsx"),
      "utf8",
    );

    expect(historyPage).toContain("scan.responder_role");
    expect(historyPage).toContain("scan.location_recorded");
    expect(historyPage).not.toContain("scan.responder_email");
    expect(historyPage).not.toContain("scan.responder_did_val");
    expect(historyPage).not.toContain("scan.location_lat");
    expect(historyPage).not.toContain("scan.location_lng");
    expect(historyPage).not.toContain("scan.location)");
  });

  it("renders bounded responder and location fields on the scan detail page", () => {
    const detailPage = fs.readFileSync(
      path.join(process.cwd(), "client/src/pages/ScanDetail.jsx"),
      "utf8",
    );

    expect(detailPage).toContain("scan.responder_role");
    expect(detailPage).toContain("scan.location_recorded");
    expect(detailPage).not.toContain("scan.responder_email");
    expect(detailPage).not.toContain("scan.responder_did_val");
    expect(detailPage).not.toContain("scan.location_lat");
    expect(detailPage).not.toContain("scan.location_lng");
    expect(detailPage).not.toContain("scan.location)");
  });
});
