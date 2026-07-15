import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = join(fileURLToPath(new URL('.', import.meta.url)), '..');
const source = (path) => readFileSync(join(appRoot, path), 'utf8');

assert.match(
  source('ADJACENT-SURFACE-INTAKE.md'),
  /not the canonical\s+EXOCHAIN\s+Rust trust fabric/i,
  'VitalLock must remain outside the canonical Rust trust fabric',
);

const cryptoSource = source('src/lib/crypto.ts');
for (const forbidden of [
  'wasm_generate_x25519_keypair',
  'wasm_ed25519_public_from_secret',
  'wasm_encrypt_message',
]) {
  assert.equal(
    cryptoSource.includes(forbidden),
    false,
    `VitalLock must not call disabled raw-secret WASM entrypoint ${forbidden}`,
  );
}

for (const path of [
  'src/components/Navigation.tsx',
  'src/pages/Login.tsx',
  'src/pages/Settings.tsx',
]) {
  assert.doesNotMatch(
    source(path),
    /Powered by EXOCHAIN|Trust Fabric:\s*EXOCHAIN|EXOCHAIN CGR Kernel ready/i,
    `${path} contains an unsupported EXOCHAIN trust claim`,
  );
}

console.log('VitalLock surface policy OK');
