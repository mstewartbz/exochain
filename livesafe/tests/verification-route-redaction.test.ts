import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("verification route redaction wiring", () => {
  it("routes email and phone verification acknowledgements through bounded helpers", () => {
    const authRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/auth.js"),
      "utf8",
    );
    const subscriberRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/subscribers.js"),
      "utf8",
    );

    const verifyEmailStart = authRoute.indexOf("router.get('/verify-email'");
    const meStart = authRoute.indexOf("router.get('/me'");
    const verifyEmailBlock = authRoute.slice(verifyEmailStart, meStart);

    const phoneRequestStart = subscriberRoute.indexOf("router.post('/phone/request'");
    const phoneConfirmStart = subscriberRoute.indexOf("router.post('/phone/confirm'");
    const moduleExportStart = subscriberRoute.indexOf("module.exports = router;");
    const phoneRequestBlock = subscriberRoute.slice(
      phoneRequestStart,
      phoneConfirmStart,
    );
    const phoneConfirmBlock = subscriberRoute.slice(
      phoneConfirmStart,
      moduleExportStart,
    );

    expect(authRoute).toContain("buildPublicEmailVerificationResponse");
    expect(subscriberRoute).toContain("buildPublicPhoneVerificationRequestResponse");
    expect(subscriberRoute).toContain("buildPublicPhoneVerificationConfirmResponse");
    expect(verifyEmailBlock).toContain("buildPublicEmailVerificationResponse({");
    expect(phoneRequestBlock).toContain("buildPublicPhoneVerificationRequestResponse({");
    expect(phoneConfirmBlock).toContain("buildPublicPhoneVerificationConfirmResponse({");
    expect(verifyEmailBlock).not.toContain("return res.json({ message: 'Email already verified', email:");
    expect(verifyEmailBlock).not.toContain("res.json({\n      message: 'Email verified successfully',");
    expect(phoneRequestBlock).not.toContain("res.json({\n      message: 'Verification code sent',");
    expect(phoneConfirmBlock).not.toContain("return res.json({ message: 'Phone already verified', phone:");
    expect(phoneConfirmBlock).not.toContain("res.json({\n      message: 'Phone verified successfully',");
  });
});
