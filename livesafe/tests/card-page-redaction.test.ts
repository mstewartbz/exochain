const fs = require("node:fs");
const path = require("node:path");

describe("card page response alignment", () => {
  it("does not render raw emergency token or scan URL fields from the card APIs", () => {
    const cardPageSource = fs.readFileSync(
      path.join(process.cwd(), "client/src/pages/Card.jsx"),
      "utf8",
    );

    expect(cardPageSource).not.toContain("cardData.card.emergency_consent_token");
    expect(cardPageSource).not.toContain("cardData.card.scan_url");
    expect(cardPageSource).not.toContain("nfcData.nfc_payload.emergency_token");
    expect(cardPageSource).toContain("cardData.card.qr_image_url");
    expect(cardPageSource).toContain("Pointer-only metadata");
  });
});
