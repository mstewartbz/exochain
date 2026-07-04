import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. trustee directory route wiring", () => {
  it("routes the unauthenticated trustee directory through a bounded redaction helper", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );
    const trusteeRoute = paceRoute.slice(
      paceRoute.indexOf("// GET /api/pace/trustees/:subscriberDid"),
      paceRoute.indexOf("// POST /api/pace/trustees/:trusteeId/send"),
    );

    expect(paceRoute).toContain("buildPublicPaceTrusteeDirectoryResponse({");
    expect(trusteeRoute).not.toContain("email: t.email");
    expect(trusteeRoute).not.toContain("invite_phone");
    expect(trusteeRoute).not.toContain("invitation_url");
    expect(trusteeRoute).not.toContain("delivery_channel");
    expect(trusteeRoute).not.toContain("email_delivery_status");
    expect(trusteeRoute).not.toContain("sms_delivery_status");
    expect(trusteeRoute).not.toContain("master_key_hash");
    expect(trusteeRoute).not.toContain("shard_encrypted");
  });
});
