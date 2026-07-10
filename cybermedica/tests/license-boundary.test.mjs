import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const rootFile = (path) => new URL(`../${path}`, import.meta.url);

test('CyberMedica carries the LiveSafe proprietary subtree boundary', async () => {
  const [license, packageJson, packageLock, readme] = await Promise.all([
    readFile(rootFile('LICENSE'), 'utf8'),
    readFile(rootFile('package.json'), 'utf8').then(JSON.parse),
    readFile(rootFile('package-lock.json'), 'utf8').then(JSON.parse),
    readFile(rootFile('README.md'), 'utf8'),
  ]);

  assert.match(license, /^CyberMedica .* Proprietary and Confidential$/m);
  assert.match(license, /the "cybermedica\/" subtree of the exochain\/exochain repository/);
  assert.match(license, /is NOT licensed under the Apache License, Version 2\.0/);
  assert.match(license, /No license, express or implied, is granted/);
  assert.match(license, /legal@exochain\.org/);
  assert.equal(packageJson.license, 'UNLICENSED');
  assert.equal(packageLock.packages[''].license, 'UNLICENSED');
  assert.match(readme, /^## License$/m);
  assert.match(readme, /proprietary and confidential/i);
  assert.match(readme, /\[LICENSE\]\(LICENSE\)/);
});
