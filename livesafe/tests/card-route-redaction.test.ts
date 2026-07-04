import { describe, expect, it } from "vitest";
import fs from "node:fs";
import path from "node:path";

describe("card route redaction wiring", () => {
  it("routes card JSON responses through bounded helpers instead of raw card rows or NFC secrets", () => {
    const cardRoute = fs.readFileSync(
      path.join(process.cwd(), "server/routes/card.js"),
      "utf8",
    );

    expect(cardRoute).toContain("buildPublicCardIssueResponse({");
    expect(cardRoute).toContain("buildPublicCardStatusResponse({");
    expect(cardRoute).toContain("buildPublicCardNfcResponse({");
    expect(cardRoute).not.toContain("return res.json({\n        card: existingCard.rows[0]");
    expect(cardRoute).not.toContain("res.status(201).json({\n      card: cardResult.rows[0]");
    expect(cardRoute).not.toContain("scan_url: responderPortalUrl");
    expect(cardRoute).not.toContain("nfc_payload: nfcData");
  });
});
