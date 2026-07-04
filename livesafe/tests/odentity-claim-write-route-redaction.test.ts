import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("0dentity claim write route redaction wiring", () => {
  it("routes import and revoke acknowledgements through bounded helpers and requires same-subscriber ownership for imports", () => {
    const odentityRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/odentity.js"),
      "utf8",
    );
    const importStart = odentityRoute.indexOf("router.post('/claims/import'");
    const revokeStart = odentityRoute.indexOf("router.post('/claims/:claimId/revoke'");
    const eventsStart = odentityRoute.indexOf("router.post('/events/record'");
    const importBlock = odentityRoute.slice(importStart, revokeStart);
    const revokeBlock = odentityRoute.slice(revokeStart, eventsStart);

    expect(odentityRoute).toContain("buildPublicOdentityClaimImportResponse(result.rows[0])");
    expect(odentityRoute).toContain(
      "buildPublicOdentityClaimRevocationResponse({",
    );
    expect(odentityRoute).toContain("router.post('/claims/import', authMiddleware");
    expect(importBlock).toContain(
      "return res.status(403).json({ error: 'Forbidden: you can only import 0dentity claims for your own subscriber account' });",
    );
    expect(importBlock).not.toContain("res.status(201).json(result.rows[0]);");
    expect(revokeBlock).not.toContain("res.json({\n      message: 'Claim revoked successfully'");
    expect(revokeBlock).not.toContain("subscriber_id:");
    expect(revokeBlock).not.toContain("credential_hash:");
  });
});
