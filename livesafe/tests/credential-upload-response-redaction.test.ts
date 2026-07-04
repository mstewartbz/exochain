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

describe("credential upload response redaction", () => {
  it("sanitizes insurance-card metadata before returning upload acknowledgements", () => {
    const sanitizeCredentialForResponse = loadSanitizeCredentialForResponse();

    const response = sanitizeCredentialForResponse({
      id: 14,
      credential_type: "insurance_card",
      title: "Aetna Insurance Card",
      carrier: "Aetna",
      member_id: "AET1234",
      group_number: "GRP-8899",
      visibility: "private",
      data_encrypted: JSON.stringify({
        file_path: "credential-raw-secret.png",
        original_name: "insurance-card.png",
        file_size: 8192,
        mime_type: "image/png",
        extraction_confidence: 0.98,
        extraction_method: "ocr-simulated",
      }),
    });

    expect(response).toEqual({
      id: 14,
      credential_type: "insurance_card",
      title: "Aetna Insurance Card",
      carrier: "Aetna",
      member_id: "AET1234",
      group_number: "GRP-8899",
      visibility: "private",
      data_encrypted: JSON.stringify({
        encrypted: false,
        original_name: "insurance-card.png",
        file_size: 8192,
        mime_type: "image/png",
        extraction_confidence: 0.98,
        extraction_method: "ocr-simulated",
      }),
    });
    expect(response.data_encrypted).not.toContain("file_path");
  });
});
