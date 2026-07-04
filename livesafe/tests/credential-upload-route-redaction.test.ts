import fs from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";

describe("credential upload route redaction wiring", () => {
  it("sanitizes insurance upload responses instead of returning raw credential rows", () => {
    const credentialsRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/credentials.js"),
      "utf8",
    );

    const insuranceUploadStart = credentialsRoute.indexOf(
      "console.log(`[Credentials] Insurance card uploaded",
    );

    expect(insuranceUploadStart).toBeGreaterThan(-1);

    const insuranceUploadBlock = credentialsRoute.slice(
      insuranceUploadStart,
      credentialsRoute.indexOf("// POST /api/credentials/advance-directive"),
    );

    expect(insuranceUploadBlock).toContain(
      "credential: sanitizeCredentialForResponse(result.rows[0])",
    );
    expect(insuranceUploadBlock).not.toContain("credential: result.rows[0],");
    expect(insuranceUploadBlock).not.toContain("filename: req.file.filename");
  });
});
