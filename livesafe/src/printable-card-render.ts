import { Buffer } from "node:buffer";

import { TRUST_SIGNAL_TOKENS, type TrustSignalState } from "./trust-signal.js";

interface PdfDocumentLike {
  fontSize(size: number): PdfDocumentLike;
  text(text: string, x?: number, y?: number, options?: Record<string, unknown>): PdfDocumentLike;
  moveDown(lines?: number): PdfDocumentLike;
  rect(x: number, y: number, width: number, height: number): PdfDocumentLike;
  stroke(color?: string): PdfDocumentLike;
  moveTo(x: number, y: number): PdfDocumentLike;
  lineTo(x: number, y: number): PdfDocumentLike;
  dash(length: number, options?: Record<string, unknown>): PdfDocumentLike;
  undash(): PdfDocumentLike;
  on(event: "data", listener: (chunk: Buffer) => void): PdfDocumentLike;
  on(event: "end", listener: () => void): PdfDocumentLike;
  end(): void;
}

type PdfDocumentConstructor = new (options?: Record<string, unknown>) => PdfDocumentLike;

const PDFDocument = require("pdfkit") as PdfDocumentConstructor;

export type PrintablePanelKey =
  | "identity"
  | "qr"
  | "medical-release"
  | "legacy-directive"
  | "rights-assertion";

export type ContactSurfaceSource = "configuration" | "imported-artifact";

export type ContactSurfaceStatus =
  | "current"
  | "obsolete"
  | "expired"
  | "replaced"
  | "revoked";

export interface PrintablePanel {
  key: PrintablePanelKey;
  title: string;
  acceptanceRequired: boolean;
  acceptedBySubscriber?: boolean;
  confirmationRef?: string;
  jurisdictionRef?: string;
}

export interface PrintedContactPolicy {
  phoneLabel: string;
  urlLabel: string;
  source: ContactSurfaceSource;
  status: ContactSurfaceStatus;
}

export interface PrintableCardRenderRequest {
  cardVersionRef: string;
  effectiveDate: string;
  generatedAt: string;
  trustState: TrustSignalState;
  subscriberDisplayName: string;
  portraitLabel: string;
  qrPointerRef: string;
  qrPolicyRef: string;
  qrPayloadMode: "pointer-only" | "embedded-sensitive";
  resolverStatus: ContactSurfaceStatus;
  generatedFromPreferences: boolean;
  includesCutGuides: boolean;
  includesFoldInstructions: boolean;
  includesFirstFoldInstruction: boolean;
  includesLegalPrivacyArea: boolean;
  legalPrivacyLabel: string;
  printedContactPolicy: PrintedContactPolicy;
  enabledPanels: PrintablePanel[];
}

export interface PrintableCardRenderDecision {
  allowed: boolean;
  reasons: string[];
  requiredEvidence: string[];
}

export interface PrintableCardPdfArtifact {
  fileName: string;
  pageCount: number;
  instructions: string[];
  visiblePanels: PrintablePanelKey[];
  pdfBytes: Buffer;
}

const REQUIRED_PANELS: PrintablePanelKey[] = ["identity", "qr"];

const PRINT_INSTRUCTIONS = [
  "Cut along the solid guide to wallet-card size.",
  "Fold inward on dashed lines with the QR panel facing out first.",
  "Verify the printed date and replace older card versions."
] as const;

function hasValue(value: string | undefined): boolean {
  return typeof value === "string" && value.trim().length > 0;
}

function sanitizeFileSegment(value: string): string {
  return value
    .trim()
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

export function validatePrintableCardRender(
  request: PrintableCardRenderRequest
): PrintableCardRenderDecision {
  const reasons = new Set<string>();
  const requiredEvidence = new Set<string>();
  const visiblePanels = new Set(request.enabledPanels.map(panel => panel.key));

  if (!hasValue(request.cardVersionRef) || !hasValue(request.effectiveDate) || !hasValue(request.generatedAt)) {
    reasons.add("Printable card render requires shared version, effective-date, and generated-at references.");
    requiredEvidence.add("Synthetic version, effective date, and generation timestamp shared across the printed packet.");
  }

  if (!hasValue(request.subscriberDisplayName) || !hasValue(request.portraitLabel)) {
    reasons.add("Printable card render requires synthetic identity labels and portrait labeling.");
    requiredEvidence.add("Synthetic printable identity labels with no raw imported subscriber data.");
  }

  for (const panelKey of REQUIRED_PANELS) {
    if (!visiblePanels.has(panelKey)) {
      reasons.add("Printable card render requires identity and QR panels.");
      requiredEvidence.add("Printable panel inventory showing identity and QR panels.");
      break;
    }
  }

  if (!request.generatedFromPreferences) {
    reasons.add("Printable card render must be generated from account preferences and emergency-card configuration.");
    requiredEvidence.add("Generation path from subscriber preferences into the printable card artifact.");
  }

  if (!request.includesCutGuides || !request.includesFoldInstructions || !request.includesFirstFoldInstruction) {
    reasons.add("Printable card render must include cut guides and fold-order instructions.");
    requiredEvidence.add("Rendered cut guides plus first-fold instructions in the printable layout.");
  }

  if (!request.includesLegalPrivacyArea || !hasValue(request.legalPrivacyLabel)) {
    reasons.add("Printable card render requires a configuration-backed legal or privacy area.");
    requiredEvidence.add("Printable legal or privacy area aligned to current access policy.");
  }

  if (request.printedContactPolicy.source !== "configuration") {
    reasons.add("Printable card contact surfaces must come from current configuration-backed values.");
    requiredEvidence.add("Generation-time contact surface lookup from current configuration.");
  }

  if (request.printedContactPolicy.status !== "current") {
    reasons.add("Printable card generation denies obsolete, expired, replaced, or revoked printed contact surfaces.");
    requiredEvidence.add("Current-status validation for all printed phone, URL, and contact surfaces.");
  }

  if (
    !hasValue(request.printedContactPolicy.phoneLabel) ||
    !hasValue(request.printedContactPolicy.urlLabel)
  ) {
    reasons.add("Printable card render requires non-empty configuration-backed phone and URL labels.");
    requiredEvidence.add("Visible printed contact labels sourced from current configuration.");
  }

  if (request.qrPayloadMode !== "pointer-only") {
    reasons.add("Printable card QR payloads must carry only retrieval or activation pointers.");
    requiredEvidence.add("QR payload review proving pointer-only printable payload content.");
  }

  if (!hasValue(request.qrPointerRef) || !hasValue(request.qrPolicyRef) || request.resolverStatus !== "current") {
    reasons.add("Printable card render requires current QR pointer and policy references.");
    requiredEvidence.add("Current QR pointer reference with generation-time policy validation.");
  }

  for (const panel of request.enabledPanels) {
    if (!hasValue(panel.title)) {
      reasons.add("Printable card panels require visible panel titles.");
      requiredEvidence.add("Panel-title inventory for the rendered packet.");
    }

    if (
      panel.acceptanceRequired &&
      (!panel.acceptedBySubscriber || !hasValue(panel.confirmationRef) || !hasValue(panel.jurisdictionRef))
    ) {
      reasons.add(
        "Printable optional medical or directive panels require subscriber acceptance, confirmation, and jurisdiction evidence."
      );
      requiredEvidence.add("Acceptance, confirmation, and jurisdiction references for each enabled optional panel.");
    }
  }

  return {
    allowed: reasons.size === 0,
    reasons: [...reasons].sort(),
    requiredEvidence: [...requiredEvidence].sort()
  };
}

export async function generatePrintableCardPdf(
  request: PrintableCardRenderRequest
): Promise<PrintableCardPdfArtifact> {
  const decision = validatePrintableCardRender(request);
  if (!decision.allowed) {
    throw new Error(`Printable card render denied: ${decision.reasons.join(" | ")}`);
  }

  const token = TRUST_SIGNAL_TOKENS[request.trustState];
  const pdfBytes = await renderPdf(request, token.displayText, token.machineText);

  return {
    fileName: `livesafe-card-${sanitizeFileSegment(request.cardVersionRef)}.pdf`,
    pageCount: 1,
    instructions: [...PRINT_INSTRUCTIONS],
    visiblePanels: request.enabledPanels.map(panel => panel.key),
    pdfBytes
  };
}

function renderPdf(
  request: PrintableCardRenderRequest,
  trustDisplayText: string,
  trustMachineText: string
): Promise<Buffer> {
  const doc = new PDFDocument({ margin: 36, size: "LETTER", compress: false });
  const chunks: Buffer[] = [];

  return new Promise((resolve, reject) => {
    doc.on("data", chunk => {
      chunks.push(chunk);
    });
    doc.on("end", () => {
      resolve(Buffer.concat(chunks));
    });

    try {
      doc.fontSize(18).text("LiveSafe Printable Emergency Card", 36, 36);
      doc.fontSize(10).text(`Version: ${request.cardVersionRef}`);
      doc.text(`Effective date: ${request.effectiveDate}`);
      doc.text(`Generated at: ${request.generatedAt}`);
      doc.text(`Trust state: ${trustDisplayText} [${trustMachineText}]`);
      doc.moveDown(0.5);
      doc.text(`Subscriber: ${request.subscriberDisplayName}`);
      doc.text(`Portrait label: ${request.portraitLabel}`);
      doc.text(`QR pointer: ${request.qrPointerRef}`);
      doc.text(`QR policy: ${request.qrPolicyRef}`);
      doc.moveDown(0.5);
      doc.text("Print instructions:");
      for (const instruction of PRINT_INSTRUCTIONS) {
        doc.text(`- ${instruction}`);
      }
      doc.moveDown(0.5);
      doc.text("Printed contact surfaces:");
      doc.text(`- ${request.printedContactPolicy.phoneLabel}`);
      doc.text(`- ${request.printedContactPolicy.urlLabel}`);
      doc.moveDown(0.5);
      doc.text("Enabled panels:");
      for (const panel of request.enabledPanels) {
        doc.text(`- ${panel.title} (${panel.key})`);
      }
      doc.moveDown(0.5);
      doc.text(`Privacy area: ${request.legalPrivacyLabel}`);

      doc.rect(36, 420, 252, 162).stroke("#1f2937");
      doc.rect(324, 420, 252, 162).stroke("#1f2937");
      doc.dash(6, { space: 4 }).moveTo(306, 420).lineTo(306, 582).stroke("#9ca3af").undash();
      doc.moveTo(36, 501).lineTo(576, 501).stroke("#9ca3af");
      doc.fontSize(10).text("CUT LINE", 40, 402);
      doc.text("FIRST FOLD", 268, 590);
      doc.text("Wallet front / identity", 52, 438);
      doc.text("Wallet back / QR", 340, 438);
      doc.text("Foldable packet panels continue inside the card body.", 52, 530, {
        width: 500
      });

      doc.end();
    } catch (error) {
      reject(error);
    }
  });
}
