const fs = require("node:fs");
const path = require("node:path");

describe("legacy subscriber DID consent read authorization boundary", () => {
  it("requires a subscriber-or-provider token before returning consent history", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );

    expect(consentRoute).toContain(
      "router.get('/:subscriberDid', subscriberOrProviderAuthMiddleware, async (req, res) => {",
    );
    expect(consentRoute).not.toContain(
      "router.get('/:subscriberDid', async (req, res) => {",
    );
  });

  it("scopes the subscriber branch to the authenticated subscriber's own DID", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );
    const legacyRoute = consentRoute.slice(
      consentRoute.indexOf("router.get('/:subscriberDid'"),
      consentRoute.indexOf("// POST /api/consent/provider"),
    );

    expect(legacyRoute).toContain("if (req.authRole === 'subscriber') {");
    expect(legacyRoute).toContain(
      "if (String(req.user.id) !== String(resolvedSubscriberId)) {",
    );
    expect(legacyRoute).toContain(
      "return res.status(403).json({ error: 'Cannot read consent history for another subscriber' });",
    );
  });

  it("scopes the provider branch to consent_events naming that provider", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );
    const legacyRoute = consentRoute.slice(
      consentRoute.indexOf("router.get('/:subscriberDid'"),
      consentRoute.indexOf("// POST /api/consent/provider"),
    );

    expect(legacyRoute).toContain(
      "SELECT * FROM consent_events WHERE subscriber_id = $1 AND provider_id = $2 ORDER BY granted_at DESC",
    );
    expect(legacyRoute).toContain("const providerId = req.user.id;");
  });

  it("defines subscriberOrProviderAuthMiddleware to fail closed without a valid bearer token", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );
    const middleware = consentRoute.slice(
      consentRoute.indexOf("function subscriberOrProviderAuthMiddleware"),
      consentRoute.indexOf("// GET /api/consent/:subscriberDid"),
    );

    expect(middleware).toContain(
      "return res.status(401).json({ error: 'Authentication required' });",
    );
    expect(middleware).toContain(
      "return res.status(403).json({ error: 'Subscriber or provider account required' });",
    );
    expect(middleware).toContain(
      "return res.status(401).json({ error: 'Invalid or expired token' });",
    );
  });
});
