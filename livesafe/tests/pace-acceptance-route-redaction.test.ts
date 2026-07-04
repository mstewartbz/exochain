import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. invitation acceptance route wiring", () => {
  it("routes acceptance responses through a bounded redaction helper", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );
    const acceptanceRoute = paceRoute.slice(
      paceRoute.indexOf("// POST /api/pace/accept-invitation"),
      paceRoute.indexOf("// GET /api/pace/vss-status/:subscriberDid"),
    );

    expect(paceRoute).toContain("buildTrusteeInvitationAcceptanceResponse({");
    expect(acceptanceRoute).not.toContain("res.status(201).json({\n      user:");
    expect(acceptanceRoute).not.toContain("email: trustee.email,\n        role: trustee.role,");
    expect(acceptanceRoute).not.toContain("shard_ref: trustee.shard_ref");
    expect(acceptanceRoute).not.toContain("master_key_hash: vssCeremony.master_key_hash");
  });
});
