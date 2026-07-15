import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = join(fileURLToPath(new URL('.', import.meta.url)), '..');
const source = (path) => readFileSync(join(appRoot, path), 'utf8');

assert.match(
  source('ADJACENT-SURFACE-INTAKE.md'),
  /not the canonical\s+EXOCHAIN\s+Rust trust fabric/i,
  'LiveSafe must remain outside the canonical Rust trust fabric',
);

for (const path of [
  'src/pages/Login.tsx',
  'src/pages/Landing.tsx',
  'src/components/Navigation.tsx',
  'src/pages/Settings.tsx',
]) {
  assert.doesNotMatch(
    source(path),
    /@\/wasm\/exochain_wasm|wasm_generate_x25519_keypair|Powered by EXOCHAIN|Trust Fabric:\s*EXOCHAIN|Secure kernel ready/i,
    `${path} calls an unproved adapter or contains an unsupported EXOCHAIN trust claim`,
  );
}

console.log('LiveSafe surface policy OK');
