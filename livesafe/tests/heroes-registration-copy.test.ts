import { readFileSync } from "node:fs";
import path from "node:path";

const root = process.cwd();
const registerPage = readFileSync(
  path.join(root, "client/src/pages/Register.jsx"),
  "utf8"
);
const settingsPage = readFileSync(
  path.join(root, "client/src/pages/Settings.jsx"),
  "utf8"
);
const authContext = readFileSync(
  path.join(root, "client/src/context/AuthContext.jsx"),
  "utf8"
);
const authRoutes = readFileSync(
  path.join(root, "server/routes/auth.js"),
  "utf8"
);
const subscriberAuthResponse = readFileSync(
  path.join(root, "server/utils/auth-subscriber-response.js"),
  "utf8"
);

describe("Heroes registration copy and payload", () => {
  it("presents the free tier as Heroes, not military-only", () => {
    expect(registerPage).toContain("I am a Hero:");
    expect(registerPage).toContain("first responder");
    expect(registerPage).toContain("law enforcement");
    expect(registerPage).toContain("Fire & Rescue");
    expect(registerPage).toContain("ER");
    expect(registerPage).toContain("FEMA/NIMS");
    expect(registerPage).toContain("powerline");
    expect(registerPage).toContain("Heroes accounts are");
    expect(settingsPage).toContain("Heroes — Free Forever");
    expect(settingsPage).not.toContain("Military/Veteran — Free Forever");
  });

  it("sends and returns is_hero while preserving the legacy is_military alias", () => {
    expect(authContext).toContain("is_hero: isHero || false");
    expect(authContext).toContain("is_military: isHero || false");
    expect(authRoutes).toContain("const { email, password, first_name, last_name, is_hero, is_military }");
    expect(authRoutes).toContain("buildPublicSubscriberAuthSessionResponse");
    expect(subscriberAuthResponse).toContain("const isHero = Boolean(user.is_hero || user.is_military);");
    expect(subscriberAuthResponse).toContain('tier: isHero ? "free_hero" : "free"');
    expect(subscriberAuthResponse).toContain('is_hero: isHero');
    expect(subscriberAuthResponse).toContain('is_military: isHero');
  });
});
