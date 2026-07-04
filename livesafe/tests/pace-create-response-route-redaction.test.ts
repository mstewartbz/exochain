import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. trustee creation route wiring", () => {
  it("uses a bounded redaction helper for trustee nomination responses", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );
    const createRoute = paceRoute.slice(
      paceRoute.indexOf("// POST /api/pace/trustees"),
      paceRoute.indexOf("// GET /api/pace/trustees/:subscriberDid"),
    );
    const responseBlock = createRoute.slice(
      createRoute.indexOf("const publicResults = results.map((result) =>"),
      createRoute.indexOf("res.status(201).json(publicResults);") +
        "res.status(201).json(publicResults);".length,
    );

    expect(createRoute).toContain("buildTrusteeInvitationSendResponse({");
    expect(createRoute).toContain("res.status(201).json(publicResults)");
    expect(responseBlock).not.toContain("res.status(201).json(results)");
    expect(responseBlock).not.toContain("invite_phone");
    expect(responseBlock).not.toContain("invitation_token");
    expect(responseBlock).not.toContain("invitation_url");
    expect(responseBlock).not.toContain("email_delivery_status");
    expect(responseBlock).not.toContain("sms_delivery_status");
    expect(responseBlock).not.toContain("delivery_error_code");
  });
});
