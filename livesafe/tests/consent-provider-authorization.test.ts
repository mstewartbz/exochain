const fs = require("node:fs");
const path = require("node:path");

describe("provider consent grant authorization boundary", () => {
  it("requires a subscriber token before consent can be granted", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );

    expect(consentRoute).toContain("const isSubscriber = decoded.user_type === 'subscriber' || decoded.role === 'subscriber';");
    expect(consentRoute).toContain("return res.status(403).json({ error: 'Subscriber account required' });");
  });

  it("binds provider consent creation to the authenticated subscriber", () => {
    const consentRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/consent.js"),
      "utf8",
    );
    const providerGrantRoute = consentRoute.slice(
      consentRoute.indexOf("router.post('/provider'"),
      consentRoute.indexOf("// POST /api/consent/request-access"),
    );

    expect(providerGrantRoute).toContain("const authenticatedSubscriberId = String(req.user.id);");
    expect(providerGrantRoute).toContain("String(subscriber_id) !== authenticatedSubscriberId");
    expect(providerGrantRoute).toContain("Cannot grant consent for another subscriber");
    expect(providerGrantRoute).toContain("SELECT id, did FROM subscribers WHERE id = $1");
    expect(providerGrantRoute).toContain("const resolvedSubscriberId = subResult.rows[0].id;");
    expect(providerGrantRoute).not.toContain("let resolvedSubscriberId = subscriber_id;");
  });
});
