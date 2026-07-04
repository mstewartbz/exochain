import { describe, expect, it } from "vitest";

import {
  type PrintableCardRenderRequest,
  generatePrintableCardPdf,
  validatePrintableCardRender
} from "../src/printable-card-render.js";

function validRenderRequest(): PrintableCardRenderRequest {
  return {
    cardVersionRef: "card:version:2026-05-26",
    effectiveDate: "2026-05-26",
    generatedAt: "2026-05-26T09:05:00Z",
    trustState: "not-verified" as const,
    subscriberDisplayName: "Alex Example",
    portraitLabel: "Synthetic portrait silhouette",
    qrPointerRef: "qr:pointer:2026-05-26",
    qrPolicyRef: "policy:qr:activation",
    qrPayloadMode: "pointer-only" as const,
    resolverStatus: "current" as const,
    generatedFromPreferences: true,
    includesCutGuides: true,
    includesFoldInstructions: true,
    includesFirstFoldInstruction: true,
    includesLegalPrivacyArea: true,
    legalPrivacyLabel: "Config-backed privacy notice",
    printedContactPolicy: {
      phoneLabel: "Configured emergency support",
      urlLabel: "Configured responder URL",
      source: "configuration",
      status: "current"
    },
    enabledPanels: [
      {
        key: "identity",
        title: "Identity + emergency contact",
        acceptanceRequired: false
      },
      {
        key: "qr",
        title: "QR activation",
        acceptanceRequired: false
      },
      {
        key: "medical-release",
        title: "Medical release",
        acceptanceRequired: true,
        acceptedBySubscriber: true,
        confirmationRef: "confirm:medical-release",
        jurisdictionRef: "jurisdiction:us-nc"
      }
    ]
  };
}

describe("printable card render contract", () => {
  it("renders a synthetic PDF packet with cut and fold instructions", async () => {
    const artifact = await generatePrintableCardPdf(validRenderRequest());

    expect(artifact.fileName).toBe("livesafe-card-card-version-2026-05-26.pdf");
    expect(artifact.pageCount).toBe(1);
    expect(artifact.instructions).toEqual([
      "Cut along the solid guide to wallet-card size.",
      "Fold inward on dashed lines with the QR panel facing out first.",
      "Verify the printed date and replace older card versions."
    ]);
    expect(artifact.visiblePanels).toEqual([
      "identity",
      "qr",
      "medical-release"
    ]);
    expect(artifact.pdfBytes.subarray(0, 5).toString("utf8")).toBe("%PDF-");
    expect(artifact.pdfBytes.length).toBeGreaterThan(1500);
  });

  it("fails closed when printed contact surfaces are not current configuration-backed values", () => {
    const request = validRenderRequest();
    request.printedContactPolicy.status = "obsolete";
    request.printedContactPolicy.source = "imported-artifact";

    const decision = validatePrintableCardRender(request);

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Printable card contact surfaces must come from current configuration-backed values."
    );
    expect(decision.reasons).toContain(
      "Printable card generation denies obsolete, expired, replaced, or revoked printed contact surfaces."
    );
  });

  it("fails closed when optional panels lack subscriber acceptance evidence", () => {
    const request = validRenderRequest();
    request.enabledPanels[2] = {
      key: "medical-release",
      title: "Medical release",
      acceptanceRequired: true
    };

    const decision = validatePrintableCardRender(request);

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Printable optional medical or directive panels require subscriber acceptance, confirmation, and jurisdiction evidence."
    );
  });
});
