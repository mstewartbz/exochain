import { describe, expect, it } from "vitest";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

describe("Docker runtime assets", () => {
  it("copies boundary config into the production image for runtime adapter reads", () => {
    const dockerfile = readFileSync(resolve(process.cwd(), "Dockerfile"), "utf8");

    expect(dockerfile).toContain("COPY config/ ./config/");
    expect(dockerfile).toContain('CMD ["node", "server/index.js"]');
  });
});
