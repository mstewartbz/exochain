import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("scan create route redaction wiring", () => {
  it("routes scan creation responses through the bounded helper", () => {
    const scanRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/scan.js"),
      "utf8",
    );

    expect(scanRoute).toContain("buildPublicScanCreateResponse({");
    expect(scanRoute).not.toContain("res.status(201).json({\n        ...scan,");
    expect(scanRoute).not.toContain("recipient_did: n.recipient_did");
  });
});
