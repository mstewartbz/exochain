import { existsSync, readFileSync } from "node:fs";
import path from "node:path";

const root = process.cwd();

function readJson<T>(relativePath: string): T {
  return JSON.parse(readFileSync(path.join(root, relativePath), "utf8")) as T;
}

interface SurfaceIntake {
  classification: string;
  constitutionalTrustClaimsAllowed: boolean;
  runtimeAdapter: {
    status: string;
    canReadExochainCoreState: boolean;
    canWriteExochainCoreState: boolean;
    canMintConsentAuthorityProvenanceOrGovernanceOutcomes: boolean;
  };
  secrets: unknown[];
}

interface PrimitiveRegistry {
  localEvidencePath: string;
  runtimeAdapterStatus: string;
  primitiveCategories: Array<{
    id: string;
    evidencePath: string;
    adapterStatus: string;
  }>;
}

describe("workspace configuration", () => {
  it("keeps surface intake public-claims boundaries fail-closed with a verified runtime adapter", () => {
    const intake = readJson<SurfaceIntake>("config/surface-intake.json");

    expect(intake.classification).toBe("adjacent-surface");
    expect(intake.constitutionalTrustClaimsAllowed).toBe(false);
    expect(intake.runtimeAdapter).toEqual({
      status: "verified",
      canReadExochainCoreState: true,
      canWriteExochainCoreState: true,
      canMintConsentAuthorityProvenanceOrGovernanceOutcomes: false
    });
    expect(intake.secrets).toEqual([]);
  });

  it("records EXOCHAIN primitive evidence as evidence-only alongside a verified runtime adapter", () => {
    const registry = readJson<PrimitiveRegistry>("config/exochain-primitives.json");

    expect(registry.runtimeAdapterStatus).toBe("verified");
    expect(registry.primitiveCategories.length).toBeGreaterThanOrEqual(10);

    const ids = new Set<string>();
    for (const primitive of registry.primitiveCategories) {
      expect(primitive.id).toMatch(/^[a-z0-9-]+$/);
      expect(primitive.evidencePath).toMatch(/^(crates|packages)\//);
      expect(primitive.adapterStatus).toBe("evidence-only");
      expect(ids.has(primitive.id)).toBe(false);
      ids.add(primitive.id);
    }
  });

  it("can validate local EXOCHAIN evidence paths when the repo is present", () => {
    const registry = readJson<PrimitiveRegistry>("config/exochain-primitives.json");
    const exochainPath =
      process.env.EXOCHAIN_REPO_PATH ?? registry.localEvidencePath;

    if (!existsSync(exochainPath)) {
      return;
    }

    for (const primitive of registry.primitiveCategories) {
      expect(existsSync(path.join(exochainPath, primitive.evidencePath))).toBe(true);
    }
  });
});
