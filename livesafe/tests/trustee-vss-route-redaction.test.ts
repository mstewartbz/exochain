import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe('trustee VSS route redaction wiring', () => {
  it('routes trustee subscriber detail responses through bounded VSS status summaries', () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), 'server/routes/pace.js'),
      'utf8',
    );

    const detailRoute = paceRoute.slice(
      paceRoute.indexOf("// GET /api/pace/subscriber/:subscriberDid/details"),
      paceRoute.indexOf("// POST /api/pace/invitation/:token/request-resend"),
    );

    expect(paceRoute).toContain("buildTrusteeVssStatusSummary(");
    expect(detailRoute).not.toContain("shard_ref: myTrustee.shard_ref");
  });

  it('routes trustee profile responses through bounded VSS status summaries', () => {
    const authRoute = fs.readFileSync(
      path.join(process.cwd(), 'server/routes/auth.js'),
      'utf8',
    );

    const trusteeProfileRoute = authRoute.slice(
      authRoute.indexOf("// GET /api/auth/trustee/me"),
      authRoute.indexOf("// =============================================================================\n// RESPONDER AUTH ROUTES"),
    );

    expect(authRoute).toContain("buildTrusteeVssStatusSummary(");
    expect(trusteeProfileRoute).not.toContain("shard_ref: r.shard_ref");
  });
});
