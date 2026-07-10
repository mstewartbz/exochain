import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("GitHub quality workflow", () => {
  it("keeps every LiveSafe package audit in the canonical quality gate", () => {
    const packageJson = JSON.parse(
      readFileSync(resolve(process.cwd(), "package.json"), "utf8"),
    );

    expect(packageJson.scripts["audit:deps"]).toBe(
      "npm audit --package-lock-only --audit-level=low && " +
        "npm --prefix client audit --package-lock-only --audit-level=low && " +
        "npm --prefix responder audit --package-lock-only --audit-level=low && " +
        "npm --prefix server audit --package-lock-only --audit-level=low",
    );
    expect(packageJson.scripts.quality).toContain("npm run audit:deps");
  });

  it("installs server dependencies before root quality tests import server routes", () => {
    const workflow = readFileSync(
      resolve(process.cwd(), "../.github/workflows/livesafe-ci.yml"),
      "utf8",
    );

    const rootInstallIndex = workflow.indexOf("run: npm ci");
    const serverInstallIndex = workflow.indexOf("run: npm --prefix server ci");
    const qualityIndex = workflow.indexOf("run: npm run quality");

    expect(workflow).toContain("livesafe/**");
    expect(workflow).toContain("workflow_call:");
    expect(workflow).toContain("commit_sha:");
    expect(workflow).toContain("permissions:");
    expect(workflow).toContain("contents: read");
    expect(workflow).toContain("ref: ${{ inputs.commit_sha || github.sha }}");
    expect(rootInstallIndex).toBeGreaterThanOrEqual(0);
    expect(serverInstallIndex).toBeGreaterThan(rootInstallIndex);
    expect(qualityIndex).toBeGreaterThan(serverInstallIndex);
    expect(workflow).toContain(
      "docker build -f livesafe/Dockerfile livesafe",
    );
    expect(workflow).not.toContain("RAILWAY_TOKEN");
    expect(workflow).not.toContain("railway up");
  });
});
