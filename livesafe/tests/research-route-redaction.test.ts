import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("research route redaction wiring", () => {
  it("routes research responses through bounded helpers instead of raw rows and raw audit details", () => {
    const researchRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/research.js"),
      "utf8",
    );

    expect(researchRoute).toContain("buildResearchOptInResponse");
    expect(researchRoute).toContain("buildResearchOptInMutationResponse");
    expect(researchRoute).toContain("buildResearchAuditTrailResponse(result.rows)");
    expect(researchRoute).toContain("buildResearchTrialConsentListResponse(result.rows)");
    expect(researchRoute).toContain("buildResearchTrialConsentResponse(existing.rows[0])");
    expect(researchRoute).toContain("buildResearchSubscriberTrialMatchResponse");
    expect(researchRoute).not.toContain("return res.json({\n        opted_in: false,");
    expect(researchRoute).not.toContain("res.json(result.rows);");
    expect(researchRoute).not.toContain("data: result.rows[0]");
    expect(researchRoute).not.toContain("subscriber_did: subscriberDid,\n      trials: matchedTrials");
    expect(researchRoute).not.toContain("res.json({\n      success: true,\n      message: 'Successfully withdrawn from trial:");
  });
});
