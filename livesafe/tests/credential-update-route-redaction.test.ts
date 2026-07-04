import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("credential update route redaction wiring", () => {
  it("sanitizes updated credential responses instead of returning raw rows", () => {
    const credentialsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/credentials.js"),
      "utf8",
    );

    expect(credentialsRoute).toContain(
      "credential: sanitizeCredentialForResponse(result.rows[0])",
    );
    expect(credentialsRoute).not.toContain(
      "res.json({ credential: result.rows[0], message: 'Credential updated successfully' });",
    );
  });
});
