import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";

const repoRoot = resolve(process.cwd(), "..");

function readRepoFile(relativePath: string): string {
  const absolutePath = resolve(repoRoot, relativePath);

  expect(existsSync(absolutePath), `${relativePath} should exist`).toBe(true);

  return readFileSync(absolutePath, "utf8");
}

describe("LiveSafe Railway CI/CD baseline", () => {
  it("declares the ARMORCLOUD LiveSafe project environments and service names without secrets", () => {
    const manifest = JSON.parse(
      readRepoFile("livesafe/config/railway-environments.json"),
    ) as {
      projectId: string;
      workspace: string;
      environments: Array<{ id: string; name: string; livesafeDomain?: string }>;
      services: Array<{ id: string; name: string }>;
      productionDomain: string;
    };

    expect(manifest).toEqual({
      projectId: "372de75b-5f44-46c2-ab70-3c3185b5d81e",
      workspace: "ARMORCLOUD",
      environments: [
        {
          id: "3dc06fb6-c3df-4fe4-8807-0da0e62e4028",
          livesafeDomain: "https://livesafe-development.up.railway.app",
          name: "development",
        },
        {
          id: "a223bc12-fbe4-430f-abce-8e3ee7c9abd3",
          livesafeDomain: "https://livesafe-staging.up.railway.app",
          name: "staging",
        },
        {
          id: "1e5153e1-15f4-4447-bf7c-029af33927fb",
          name: "production",
        },
      ],
      services: [
        { id: "8ed3bd1a-f872-4e22-9a39-ac38953fae26", name: "livesafe" },
        { id: "4d8384d3-be5d-48d6-a914-97eb6133e53d", name: "exochain-node" },
        { id: "2ab3f445-d6f7-4245-940c-985a14e974f9", name: "exochain-node-db" },
        { id: "691122bb-025b-463d-8033-7f94f7678748", name: "Postgres" },
      ],
      productionDomain: "https://livesafe.ai",
    });
  });

  it("adds a protected Railway promotion workflow for development, staging, and production", () => {
    const workflow = readRepoFile(".github/workflows/livesafe-railway-deploy.yml");

    expect(workflow).toContain("name: LiveSafe Railway Deploy");
    expect(workflow).toContain(
      "RAILWAY_PROJECT_ID: 372de75b-5f44-46c2-ab70-3c3185b5d81e",
    );
    expect(workflow).toContain(
      "RAILWAY_DEVELOPMENT_ENVIRONMENT_ID: 3dc06fb6-c3df-4fe4-8807-0da0e62e4028",
    );
    expect(workflow).toContain(
      "RAILWAY_STAGING_ENVIRONMENT_ID: a223bc12-fbe4-430f-abce-8e3ee7c9abd3",
    );
    expect(workflow).toContain(
      "RAILWAY_PRODUCTION_ENVIRONMENT_ID: 1e5153e1-15f4-4447-bf7c-029af33927fb",
    );
    expect(workflow).toContain(
      "EXOCHAIN_NODE_SERVICE_ID: 4d8384d3-be5d-48d6-a914-97eb6133e53d",
    );
    expect(workflow).toContain(
      "LIVESAFE_SERVICE_ID: 8ed3bd1a-f872-4e22-9a39-ac38953fae26",
    );
    expect(workflow).toContain("secrets.RAILWAY_TOKEN");
    expect(workflow).toContain("environment: livesafe-development");
    expect(workflow).toContain("environment: livesafe-staging");
    expect(workflow).toContain("environment: livesafe-production");
    expect(workflow).toContain("verify-livesafe:");
    expect(workflow).toContain("uses: ./.github/workflows/livesafe-ci.yml");
    expect(workflow).toContain("commit_sha: ${{ inputs.commit_sha || github.sha }}");
    expect(workflow).toContain("needs: verify-livesafe");
    expect(workflow).toContain(
      'railway up . --path-as-root --project "$RAILWAY_PROJECT_ID" --environment "$RAILWAY_DEVELOPMENT_ENVIRONMENT_ID" --service "$EXOCHAIN_NODE_SERVICE_ID"',
    );
    expect(workflow).toContain(
      'railway up livesafe --path-as-root --project "$RAILWAY_PROJECT_ID" --environment "$RAILWAY_DEVELOPMENT_ENVIRONMENT_ID" --service "$LIVESAFE_SERVICE_ID"',
    );
    expect(workflow).toContain(
      'railway up . --path-as-root --project "$RAILWAY_PROJECT_ID" --environment "$TARGET_ENVIRONMENT_ID" --service "$EXOCHAIN_NODE_SERVICE_ID"',
    );
    expect(workflow).toContain(
      'railway up livesafe --path-as-root --project "$RAILWAY_PROJECT_ID" --environment "$TARGET_ENVIRONMENT_ID" --service "$LIVESAFE_SERVICE_ID"',
    );
    expect(workflow).toContain(
      'scripts/livesafe-railway-smoke.sh "$TARGET_ENVIRONMENT"',
    );
  });

  it("provides a bounded smoke probe script that never prints Railway variables", () => {
    const script = readRepoFile("scripts/livesafe-railway-smoke.sh");

    expect(script).toContain(
      'railway service list --project "$RAILWAY_PROJECT_ID" --environment "$railway_environment_id" --json',
    );
    expect(script).toContain('railway_environment_id="3dc06fb6-c3df-4fe4-8807-0da0e62e4028"');
    expect(script).toContain('railway_environment_id="a223bc12-fbe4-430f-abce-8e3ee7c9abd3"');
    expect(script).toContain('railway_environment_id="1e5153e1-15f4-4447-bf7c-029af33927fb"');
    expect(script).toContain("deadline_seconds=");
    expect(script).toContain("while [ \"$SECONDS\" -lt \"$deadline_seconds\" ]; do");
    expect(script).toContain("sleep 10");
    expect(script).toContain('curl -fsS "$livesafe_url/api/health"');
    expect(script).toContain('curl -fsS "$livesafe_url/api/trust/status"');
    expect(script).toContain("public_claims_allowed == false");
    expect(script).not.toContain("railway variable list");
    expect(script).not.toContain("--kv");
  });
});
