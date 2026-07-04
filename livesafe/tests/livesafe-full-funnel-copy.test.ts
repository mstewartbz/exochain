import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

const root = process.cwd();

function read(relativePath: string): string {
  return readFileSync(path.join(root, relativePath), "utf8");
}

describe("LiveSafe full-funnel onboarding copy", () => {
  const funnelFiles = [
    "client/src/pages/Home.jsx",
    "client/src/pages/Register.jsx",
    "client/src/pages/Login.jsx",
    "client/src/pages/OnboardingWizard.jsx",
    "client/src/pages/Dashboard.jsx",
    "client/src/pages/Pace.jsx",
    "client/src/pages/Card.jsx",
    "client/src/pages/TrusteeAccept.jsx",
  ];

  it("anchors the public and authenticated funnel on the Safety Circle loop", () => {
    const content = funnelFiles.map(read).join("\n");

    expect(read("client/src/pages/Home.jsx")).toContain(
      "Create your card. Invite your four. Protect your people.",
    );
    expect(content).toContain("Safety Circle");
    expect(content).toContain("Primary");
    expect(content).toContain("Alternate");
    expect(content).toContain("Contingent");
    expect(content).toContain("Emergency");
    expect(content).toContain("Complete your Safety Circle and receive 4 months of Plus");
  });

  it("makes P.A.C.E. invitation channels visible in onboarding and circle management", () => {
    const onboarding = read("client/src/pages/OnboardingWizard.jsx");
    const pace = read("client/src/pages/Pace.jsx");

    for (const content of [onboarding, pace]) {
      expect(content).toContain("Email");
      expect(content).toContain("SMS");
      expect(content).toContain("Copy link");
    }
  });

  it("keeps invitee autonomy and privacy boundaries visible", () => {
    const invitee = read("client/src/pages/TrusteeAccept.jsx");

    expect(invitee).toContain("This is not a marketing invite.");
    expect(invitee).toContain("Accepting this role does not give you");
    expect(invitee).toContain("You can accept, decline, or ask");
    expect(invitee).toContain("revoke later");
  });

  it("removes legacy Custodial role copy from the user-facing funnel", () => {
    const content = funnelFiles.map(read).join("\n");

    expect(content).not.toMatch(/Custodial|custodial/);
  });
});
