import { describe, expect, it } from "vitest";

import {
  buildFeedbackCodeHints,
  feedbackCodeHintComponents,
  type FeedbackCodeHintComponent
} from "../src/feedback-code-hints.js";

describe("feedback code-hints registry contract", () => {
  it("builds bounded code hints for known UI components", () => {
    expect(
      buildFeedbackCodeHints("ice-card-generator", {
        service: "api",
        filePaths: [
          "src/ice_card_packet.rs",
          "src/printable-card-render.ts",
          "server/routes/card.js"
        ],
        specRef: "docs/context/LIVESAFE_QR_ACTIVATION_MODEL.md",
        storageKeys: ["livesafe:qr:current", "livesafe:card:print:current"],
        apiOperation: "generate-ice-card-packet"
      })
    ).toEqual({
      component: "ice-card-generator",
      service: "api",
      filePaths: [
        "src/ice_card_packet.rs",
        "src/printable-card-render.ts",
        "server/routes/card.js"
      ],
      specRef: "docs/context/LIVESAFE_QR_ACTIVATION_MODEL.md",
      storageKeys: ["livesafe:qr:current", "livesafe:card:print:current"],
      apiOperation: "generate-ice-card-packet"
    });
  });

  it("exposes the source-backed UI component vocabulary", () => {
    const supported: FeedbackCodeHintComponent[] = [
      "onboarding-wizard",
      "pace-invite-flow",
      "ice-card-generator",
      "qr-activation",
      "responder-view",
      "emergency-profile-editor",
      "medical-jacket",
      "consent-controls",
      "marketplace-template-config",
      "entitlement-plan-selector",
      "frontline-eligibility",
      "trust-state-banner"
    ];

    expect(feedbackCodeHintComponents).toEqual(supported);
  });

  it("fails closed for unsupported components, unsafe paths, or malformed tokens", () => {
    expect(() =>
      buildFeedbackCodeHints("feedback-sidebar", {
        filePaths: ["src/feedback-board-query.ts"]
      })
    ).toThrow("Unsupported feedback code-hint component: feedback-sidebar.");

    expect(() =>
      buildFeedbackCodeHints("trust-state-banner", {
        filePaths: ["../src/trust-signal.ts"]
      })
    ).toThrow(
      "feedback code-hint filePaths entries must stay within the LiveSafe repo and cannot traverse directories."
    );

    expect(() =>
      buildFeedbackCodeHints("trust-state-banner", {
        service: "api gateway"
      })
    ).toThrow(
      "feedback code-hint service must match /^[A-Za-z0-9:_-]+$/."
    );

    expect(() =>
      buildFeedbackCodeHints("trust-state-banner", {
        storageKeys: ["livesafe:trust current"]
      })
    ).toThrow(
      "feedback code-hint storageKeys entries must match /^[A-Za-z0-9:_-]+$/."
    );
  });
});
