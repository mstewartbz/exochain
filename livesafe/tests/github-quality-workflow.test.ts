import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

describe("GitHub quality workflow", () => {
  it("installs server dependencies before root quality tests import server routes", () => {
    const workflow = readFileSync(
      resolve(process.cwd(), "../.github/workflows/livesafe-ci.yml"),
      "utf8",
    );

    const rootInstallIndex = workflow.indexOf("run: npm ci");
    const serverInstallIndex = workflow.indexOf("run: npm --prefix server ci");
    const qualityIndex = workflow.indexOf("run: npm run quality");

    expect(workflow).toContain("livesafe/**");
    expect(rootInstallIndex).toBeGreaterThanOrEqual(0);
    expect(serverInstallIndex).toBeGreaterThan(rootInstallIndex);
    expect(qualityIndex).toBeGreaterThan(serverInstallIndex);
  });
});
