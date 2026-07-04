import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("P.A.C.E. invitation route wiring", () => {
  it("uses bounded redaction helpers for unauthenticated invitation validate and decline responses", () => {
    const paceRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/pace.js"),
      "utf8",
    );
    const validateRoute = paceRoute.slice(
      paceRoute.indexOf("// GET /api/pace/invitation/:token - Validate"),
      paceRoute.indexOf("// POST /api/pace/invitation/:token/decline"),
    );
    const declineRoute = paceRoute.slice(
      paceRoute.indexOf("// POST /api/pace/invitation/:token/decline"),
      paceRoute.indexOf("// POST /api/pace/accept-invitation"),
    );

    expect(paceRoute).toContain("buildTrusteeInvitationValidateResponse({");
    expect(paceRoute).toContain("buildTrusteeInvitationDeclineResponse({");
    expect(validateRoute).not.toContain("email: invitation.email");
    expect(validateRoute).not.toContain("subscriber_name:");
    expect(declineRoute).not.toContain("subscriber_name:");
    expect(declineRoute).not.toContain(
      "You declined the ${roleInfo.name} role for ${subscriberName}.",
    );
  });
});
