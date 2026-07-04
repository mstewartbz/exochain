import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

function loadSanitizeCredentialForResponse() {
  const credentialsRoute = fs.readFileSync(
    path.join(process.cwd(), "server/routes/credentials.js"),
    "utf8",
  );
  const match = credentialsRoute.match(
    /function sanitizeCredentialForResponse\(credential\) \{[\s\S]*?\n\}/,
  );

  if (!match) {
    throw new Error("sanitizeCredentialForResponse definition not found");
  }

  return new Function(`${match[0]}; return sanitizeCredentialForResponse;`)() as (
    credential: Record<string, unknown>,
  ) => Record<string, unknown>;
}

describe("credential update response redaction", () => {
  it("sanitizes advance directive metadata before returning credential updates", () => {
    const sanitizeCredentialForResponse = loadSanitizeCredentialForResponse();

    const response = sanitizeCredentialForResponse({
      id: 22,
      credential_type: "advance_directive",
      title: "Living Will",
      data_encrypted: JSON.stringify({
        encrypted: true,
        algorithm: "AES-256-GCM",
        original_filename: "advance-directive.pdf",
        file_path: "encrypted/raw-secret.vault",
        file_size_original: 4096,
        upload_date: "2026-06-06T11:00:00.000Z",
        document_date: "2026-06-01",
        description: "Advance directive",
        notary_info: "Notary block",
        subscriber_did: "did:exo:subscriber:41",
        custody_receipt_id: "custody-123",
      }),
    });

    expect(response).toEqual({
      id: 22,
      credential_type: "advance_directive",
      title: "Living Will",
      data_encrypted: JSON.stringify({
        encrypted: true,
        algorithm: "AES-256-GCM",
        original_filename: "advance-directive.pdf",
        file_size_original: 4096,
        upload_date: "2026-06-06T11:00:00.000Z",
        document_date: "2026-06-01",
        description: "Advance directive",
        notary_info: "Notary block",
        subscriber_did: "did:exo:subscriber:41",
        custody_receipt_id: "custody-123",
        encryption_verified: true,
      }),
    });
    expect(response.data_encrypted).not.toContain("file_path");
  });
});
