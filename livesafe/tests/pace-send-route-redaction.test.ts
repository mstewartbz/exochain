import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. invitation send route wiring", () => {
  it("uses a bounded redaction helper for invitation send and resend responses", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );
    const sendRoute = paceRoute.slice(
      paceRoute.indexOf("// POST /api/pace/trustees/:trusteeId/send"),
      paceRoute.indexOf("// GET /api/pace/invitation/:token"),
    );
    const responseBlock = sendRoute.slice(
      sendRoute.indexOf("res.json(buildTrusteeInvitationSendResponse({"),
      sendRoute.indexOf("}));") + 4,
    );

    expect(paceRoute).toContain("buildTrusteeInvitationSendResponse({");
    expect(responseBlock).not.toContain("...updateResult.rows[0]");
    expect(responseBlock).not.toContain("invitation_token");
    expect(responseBlock).not.toContain("invitation_url");
    expect(responseBlock).not.toContain("provider_message_id");
    expect(responseBlock).not.toContain("email");
    expect(responseBlock).not.toContain("subscriber_id");
  });
});
