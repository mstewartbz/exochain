import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity export route redaction wiring", () => {
  it("routes export VC responses through the bounded helper instead of mapping raw claim rows inline", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );
    const exportStart = odentityRoute.indexOf("router.get('/me/export-vc'");
    const exportEnd = odentityRoute.indexOf("module.exports = router;");
    const exportBlock = odentityRoute.slice(exportStart, exportEnd);

    expect(odentityRoute).toContain("buildPublicOdentityExportCredential({");
    expect(exportBlock).not.toContain("claims: claimsResult.rows.map");
    expect(exportBlock).not.toContain("issuer: c.issuer");
    expect(exportBlock).not.toContain("issuanceDate: c.issued_at");
    expect(exportBlock).not.toContain("id: `urn:livesafe:claim:${c.id}`");
  });
});
