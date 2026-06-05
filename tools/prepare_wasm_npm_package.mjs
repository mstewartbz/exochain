#!/usr/bin/env node
// Copyright 2026 Exochain Foundation
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at:
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// SPDX-License-Identifier: Apache-2.0

import fs from 'node:fs';
import path from 'node:path';

const packageDir = process.argv[2];
if (!packageDir) {
  console.error('usage: prepare_wasm_npm_package.mjs <wasm-package-dir>');
  process.exit(1);
}

const packageJsonPath = path.join(packageDir, 'package.json');
const licensePath = path.join(packageDir, 'LICENSE');
if (!fs.existsSync(packageJsonPath)) {
  console.error(`${packageJsonPath} is missing`);
  process.exit(1);
}
if (!fs.existsSync(licensePath)) {
  console.error(`${licensePath} is missing`);
  process.exit(1);
}

const manifest = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
if (manifest.name !== '@exochain/exochain-wasm') {
  console.error(`WASM package name must be @exochain/exochain-wasm, got ${manifest.name}`);
  process.exit(1);
}
if (manifest.license !== 'Apache-2.0') {
  console.error(`WASM package license must be Apache-2.0, got ${manifest.license}`);
  process.exit(1);
}
if (manifest.private === true) {
  console.error('WASM npm package must not be private');
  process.exit(1);
}

const requiredFiles = [
  'LICENSE',
  'exochain_wasm.d.ts',
  'exochain_wasm.js',
  'exochain_wasm_bg.wasm',
];
const files = Array.isArray(manifest.files) ? manifest.files : [];
manifest.files = requiredFiles;
for (const required of requiredFiles) {
  if (!files.includes(required) && required !== 'LICENSE') {
    console.error(`WASM package files list is missing ${required}`);
    process.exit(1);
  }
}
if (manifest.main !== 'exochain_wasm.js') {
  console.error(`WASM package main must be exochain_wasm.js, got ${manifest.main}`);
  process.exit(1);
}
if (manifest.types !== 'exochain_wasm.d.ts') {
  console.error(`WASM package types must be exochain_wasm.d.ts, got ${manifest.types}`);
  process.exit(1);
}

fs.writeFileSync(packageJsonPath, `${JSON.stringify(manifest, null, 2)}\n`);
