import { existsSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(fileURLToPath(new URL("..", import.meta.url)));

const ignoredDirectories = new Set([
  ".git",
  ".vitest",
  "coverage",
  "dist",
  "node_modules",
  "target",
  "tmp"
]);

const textExtensions = new Set([
  ".json",
  ".md",
  ".mjs",
  ".rs",
  ".toml",
  ".ts",
  ".yaml",
  ".yml"
]);

const markerPatterns = [
  /\bTODO\b/i,
  /\bTBD\b/i,
  /\bFIXME\b/i,
  /\bXXX\b/i,
  /\bfuture phase\b/i,
  /\bpostpone\b/i,
  /\bstubbed\b/i
];

const trustClaimPatterns = [
  /\bconstitutionally protected\b/i,
  /\bEXOCHAIN protected\b/i,
  /\bEXOCHAIN-enforced\b/i,
  /\bproduction trust active\b/i,
  /\blegally admissible\b/i,
  /\bguaranteed by EXOCHAIN\b/i
];

async function collectFiles(directory) {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const absolutePath = path.join(directory, entry.name);
    const relativePath = path.relative(root, absolutePath);

    if (entry.isDirectory()) {
      if (!ignoredDirectories.has(entry.name)) {
        files.push(...(await collectFiles(absolutePath)));
      }
      continue;
    }

    if (textExtensions.has(path.extname(entry.name))) {
      files.push(relativePath);
    }
  }

  return files;
}

function shouldScanForMarkers(relativePath) {
  if (relativePath === "AGENTS.md") {
    return false;
  }

  if (relativePath === "scripts/validate-context.mjs") {
    return false;
  }

  return /^(src|tests|docs|config|context|decisions|\.github)\//.test(relativePath) ||
    relativePath === "README.md" ||
    relativePath === "package.json";
}

function shouldScanForTrustClaims(relativePath) {
  if (relativePath === "docs/EXOCHAIN_APP_BOUNDARY.md") {
    return false;
  }

  return relativePath === "README.md" ||
    /^docs\/.*\.md$/.test(relativePath) ||
    /^context\/canon\/.*\.md$/.test(relativePath) ||
    /^src\/.*\.ts$/.test(relativePath);
}

function lineNumberFor(content, index) {
  return content.slice(0, index).split("\n").length;
}

function assertNoPatternMatches({ relativePath, content, patterns, label }) {
  for (const pattern of patterns) {
    const match = pattern.exec(content);
    if (match?.index !== undefined) {
      const line = lineNumberFor(content, match.index);
      throw new Error(`${label} in ${relativePath}:${line}: ${match[0]}`);
    }
  }
}

async function validateTextFiles() {
  const files = await collectFiles(root);

  for (const relativePath of files) {
    const absolutePath = path.join(root, relativePath);
    const content = await readFile(absolutePath, "utf8");

    if (content.length > 0 && !content.endsWith("\n")) {
      throw new Error(`Missing trailing newline in ${relativePath}`);
    }

    if (shouldScanForMarkers(relativePath)) {
      assertNoPatternMatches({
        relativePath,
        content,
        patterns: markerPatterns,
        label: "Unresolved work marker"
      });
    }

    if (shouldScanForTrustClaims(relativePath)) {
      assertNoPatternMatches({
        relativePath,
        content,
        patterns: trustClaimPatterns,
        label: "Unsupported EXOCHAIN trust claim"
      });
    }
  }
}

async function validateCanonicalContextRecords() {
  const canonDirectory = path.join(root, "context", "canon");

  if (!existsSync(canonDirectory)) {
    throw new Error("Missing context/canon directory");
  }

  const entries = await readdir(canonDirectory, { withFileTypes: true });
  for (const entry of entries) {
    if (!entry.isFile() || entry.name === "README.md" || entry.name === ".gitkeep") {
      continue;
    }

    const relativePath = path.join("context", "canon", entry.name);
    const content = await readFile(path.join(root, relativePath), "utf8");
    const requiredSections = [
      "## Source Basis",
      "## Fact vs Inference",
      "## Artifact Inventory",
      "## Open Conflicts"
    ];

    for (const section of requiredSections) {
      if (!content.includes(section)) {
        throw new Error(`Missing ${section} in ${relativePath}`);
      }
    }
  }
}

async function validatePrimitiveRegistry() {
  const registryPath = path.join(root, "config", "exochain-primitives.json");
  const registry = JSON.parse(await readFile(registryPath, "utf8"));
  const exochainPath = process.env.EXOCHAIN_REPO_PATH ?? registry.localEvidencePath;

  if (!exochainPath || !existsSync(exochainPath)) {
    console.warn("EXOCHAIN evidence path unavailable; skipped local path checks.");
    return;
  }

  for (const primitive of registry.primitiveCategories) {
    const evidencePath = path.join(exochainPath, primitive.evidencePath);
    if (!existsSync(evidencePath)) {
      throw new Error(`Missing EXOCHAIN evidence path: ${evidencePath}`);
    }
  }
}

await validateTextFiles();
await validateCanonicalContextRecords();
await validatePrimitiveRegistry();

console.log("context lint passed");
