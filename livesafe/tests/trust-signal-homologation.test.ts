import {
  evaluateTrustSignalHomologation,
  TRUST_SIGNAL_HOLON_CONTEXTS,
  TRUST_SIGNAL_MODALITIES
} from "../src/trust-signal.js";

describe("LiveSafe trust signal homologation", () => {
  it("defines geographic, ethnographic, device, and holonic modalities", () => {
    expect(TRUST_SIGNAL_MODALITIES).toEqual({
      jurisdictional: ["country", "region", "subdivision", "legal-regime"],
      geographic: ["region", "locale", "script", "text-direction"],
      linguistic: ["language", "script", "terminology", "reading-level"],
      ethnographic: [
        "plain-language",
        "cultural-symbol-review",
        "non-color-only-status",
        "assistive-technology"
      ],
      device: ["mobile", "tablet", "desktop", "print", "api"],
      holonic: TRUST_SIGNAL_HOLON_CONTEXTS
    });

    expect(TRUST_SIGNAL_HOLON_CONTEXTS).toEqual([
      "individual",
      "family",
      "pace-network",
      "responder",
      "organization",
      "agent"
    ]);
  });

  it("allows homologated mobile output when status meaning is preserved", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "not-verified",
      device: "mobile",
      holonContext: "individual",
      localeTag: "en-US",
      languageTag: "en",
      jurisdictionCode: "US",
      regionCode: "US",
      scriptCode: "Latn",
      textDirection: "ltr",
      preservesMachineState: true,
      preservesDisplayMeaning: true,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: true,
      hasNonColorOnlyStatus: true,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 44,
      layoutStable: true
    });

    expect(decision).toEqual({
      allowed: true,
      reasons: [],
      requiredEvidence: []
    });
  });

  it("denies mobile and tablet output with undersized trust controls", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "genesis-pending",
      device: "tablet",
      holonContext: "family",
      localeTag: "en-US",
      languageTag: "en",
      jurisdictionCode: "US",
      regionCode: "US",
      scriptCode: "Latn",
      textDirection: "ltr",
      preservesMachineState: true,
      preservesDisplayMeaning: true,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: true,
      hasNonColorOnlyStatus: true,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 36,
      layoutStable: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Mobile and tablet trust controls require at least 44px touch targets."
    );
  });

  it("denies culturally unreviewed or color-only status adaptations", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "internal-proof",
      device: "desktop",
      holonContext: "pace-network",
      localeTag: "es-US",
      languageTag: "es",
      jurisdictionCode: "US",
      regionCode: "US",
      scriptCode: "Latn",
      textDirection: "ltr",
      preservesMachineState: true,
      preservesDisplayMeaning: true,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: false,
      hasNonColorOnlyStatus: false,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 0,
      layoutStable: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Trust symbols require cultural-symbol review for the target audience."
    );
    expect(decision.reasons).toContain(
      "Trust state cannot rely on color alone."
    );
  });

  it("denies homologation when machine state or display meaning drifts", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "not-verified",
      device: "print",
      holonContext: "responder",
      localeTag: "en-US",
      languageTag: "en",
      jurisdictionCode: "US",
      regionCode: "US",
      scriptCode: "Latn",
      textDirection: "ltr",
      preservesMachineState: false,
      preservesDisplayMeaning: false,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: true,
      hasNonColorOnlyStatus: true,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 0,
      layoutStable: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Homologated trust output must preserve the canonical machine state."
    );
    expect(decision.reasons).toContain(
      "Homologated trust output must preserve the canonical display meaning."
    );
  });

  it("requires stable layout for holonic trust displays", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "externally-verified",
      device: "desktop",
      holonContext: "organization",
      localeTag: "en-US",
      languageTag: "en",
      jurisdictionCode: "US",
      regionCode: "US",
      scriptCode: "Latn",
      textDirection: "ltr",
      preservesMachineState: true,
      preservesDisplayMeaning: true,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: true,
      hasNonColorOnlyStatus: true,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 0,
      layoutStable: false
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Holonic trust displays require stable layout across context levels."
    );
  });

  it("allows Japanese Kanji script when canonical trust meaning is preserved", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "not-verified",
      device: "mobile",
      holonContext: "individual",
      localeTag: "ja-JP",
      languageTag: "ja",
      jurisdictionCode: "JP",
      regionCode: "JP",
      scriptCode: "Jpan",
      textDirection: "ltr",
      preservesMachineState: true,
      preservesDisplayMeaning: true,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: true,
      hasNonColorOnlyStatus: true,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 44,
      layoutStable: true
    });

    expect(decision.allowed).toBe(true);
  });

  it("denies unsupported script codes and missing jurisdiction", () => {
    const decision = evaluateTrustSignalHomologation({
      state: "genesis-pending",
      device: "api",
      holonContext: "agent",
      localeTag: "ja-JP",
      languageTag: "ja",
      jurisdictionCode: "",
      regionCode: "JP",
      scriptCode: "Emoji",
      textDirection: "ltr",
      preservesMachineState: true,
      preservesDisplayMeaning: true,
      hasLocalizedStatusText: true,
      hasCulturalSymbolReview: true,
      hasNonColorOnlyStatus: true,
      supportsAssistiveTechnology: true,
      minTouchTargetPx: 0,
      layoutStable: true
    });

    expect(decision.allowed).toBe(false);
    expect(decision.reasons).toContain(
      "Trust homologation requires a jurisdiction code."
    );
    expect(decision.reasons).toContain(
      "Trust homologation requires a supported writing system."
    );
  });
});
