import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const appRoot = join(fileURLToPath(new URL('.', import.meta.url)), '..');
const source = (path) => readFileSync(join(appRoot, path), 'utf8');

assert.match(
  source('ADJACENT-SURFACE-INTAKE.md'),
  /not the canonical\s+EXOCHAIN\s+Rust trust fabric/i,
  'CrossChecked must remain outside the canonical Rust trust fabric',
);

for (const path of ['index.html', 'src/pages/Landing.tsx', 'src/pages/Settings.tsx']) {
  assert.doesNotMatch(
    source(path),
    /EXOCHAIN\s+Trust Fabric|EXOCHAIN CGR Kernel|Powered by\s*<|Constitutional governance/i,
    `${path} contains an unsupported EXOCHAIN trust claim`,
  );
}

console.log('CrossChecked surface policy OK');
