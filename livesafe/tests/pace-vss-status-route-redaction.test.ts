import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. VSS status route wiring", () => {
  it("routes public VSS status through a bounded redaction helper", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );
    const vssRoute = paceRoute.slice(
      paceRoute.indexOf("// GET /api/pace/vss-status/:subscriberDid"),
      paceRoute.indexOf("// ─── Trustee Replacement Workflow"),
    );

    expect(paceRoute).toContain("buildPublicPaceVssStatusResponse({");
    expect(vssRoute).not.toContain("email: t.email");
    expect(vssRoute).not.toContain("shard_ref: t.shard_ref");
    expect(vssRoute).not.toContain("master_key_hash");
    expect(vssRoute).not.toContain("triggered_by");
  });
});
