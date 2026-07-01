#!/usr/bin/env node
/*
 * Copyright 2026 Exochain Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at:
 *
 *     https://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
 * SPDX-License-Identifier: Apache-2.0
 */

import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

const allowedOwners = new Set(
  (process.env.EXOCHAIN_CRATES_IO_ALLOWED_OWNERS || "exochain,exochain-foundation,bob-stewart")
    .split(",")
    .map((owner) => owner.trim().toLowerCase())
    .filter(Boolean),
);

if (allowedOwners.size === 0) {
  throw new Error("EXOCHAIN_CRATES_IO_ALLOWED_OWNERS must contain at least one crates.io owner login");
}

const metadata = JSON.parse(
  execFileSync("cargo", ["metadata", "--no-deps", "--format-version", "1"], {
    encoding: "utf8",
    maxBuffer: 20 * 1024 * 1024,
  }),
);

const workspacePackageIds = new Set(metadata.workspace_members);
const packageNames = metadata.packages
  .filter((pkg) => workspacePackageIds.has(pkg.id))
  .filter((pkg) => pkg.manifest_path.includes("/crates/"))
  .map((pkg) => pkg.name)
  .sort();

const fixtureDir = process.env.EXOCHAIN_CRATES_IO_FIXTURE_DIR;

async function loadPublishedCrateOwners(crateName) {
  if (fixtureDir) {
    const fixturePath = path.join(fixtureDir, `${crateName}.json`);
    if (!fs.existsSync(fixturePath)) {
      return null;
    }
    return JSON.parse(fs.readFileSync(fixturePath, "utf8"));
  }

  const response = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(crateName)}`, {
    headers: {
      "User-Agent": "exochain-release-namespace-guard (https://github.com/exochain/exochain)",
    },
  });
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`crates.io lookup for ${crateName} failed: HTTP ${response.status}`);
  }
  await response.json();

  const ownersResponse = await fetch(`https://crates.io/api/v1/crates/${encodeURIComponent(crateName)}/owners`, {
    headers: {
      "User-Agent": "exochain-release-namespace-guard (https://github.com/exochain/exochain)",
    },
  });
  if (!ownersResponse.ok) {
    throw new Error(`crates.io owner lookup for ${crateName} failed: HTTP ${ownersResponse.status}`);
  }
  return ownersResponse.json();
}

const failures = [];

for (const crateName of packageNames) {
  if (!crateName.startsWith("exochain-")) {
    failures.push(`${crateName} does not use the exochain-* namespace`);
    continue;
  }

  const payload = await loadPublishedCrateOwners(crateName);
  if (!payload) {
    continue;
  }

  const ownerLogins = (payload.users || [])
    .map((user) => String(user.login || "").toLowerCase())
    .filter(Boolean);
  const hasAllowedOwner = ownerLogins.some((login) => allowedOwners.has(login));

  if (!hasAllowedOwner) {
    failures.push(`${crateName} already exists on crates.io and is owned by [${ownerLogins.join(", ")}], not an approved EXOCHAIN owner`);
  }
}

if (failures.length > 0) {
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`crates.io namespace ownership guard passed for ${packageNames.length} package(s)`);
