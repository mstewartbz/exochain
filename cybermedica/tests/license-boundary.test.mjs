import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const rootFile = (path) => new URL(`../${path}`, import.meta.url);

test('CyberMedica carries the LiveSafe proprietary subtree boundary', async () => {
  const [license, packageJson, packageLock, readme, repositoryReadme] = await Promise.all([
    readFile(rootFile('LICENSE'), 'utf8'),
    readFile(rootFile('package.json'), 'utf8').then(JSON.parse),
    readFile(rootFile('package-lock.json'), 'utf8').then(JSON.parse),
    readFile(rootFile('README.md'), 'utf8'),
    readFile(new URL('../../README.md', import.meta.url), 'utf8'),
  ]);

  assert.match(license, /^CyberMedica .* Proprietary and Confidential$/m);
  assert.match(license, /the "cybermedica\/" subtree of the exochain\/exochain repository/);
  assert.match(license, /is NOT licensed\s+under the Apache License, Version 2\.0/);
  assert.match(license, /No license, express or implied, is granted/);
  assert.match(license, /legal@exochain\.org/);
  assert.equal(packageJson.license, 'UNLICENSED');
  assert.equal(packageLock.packages[''].license, 'UNLICENSED');
  assert.match(readme, /^## License$/m);
  assert.match(readme, /proprietary and confidential/i);
  assert.match(readme, /\[LICENSE\]\(LICENSE\)/);

  const apacheSpdx = ['SPDX-License-Identifier', 'Apache-2.0'].join(': ');
  const apacheGrant = ['Licensed under the', 'Apache License'].join(' ');
  const apacheMarkers = spawnSync(
    'git',
    ['grep', '-n', '-e', apacheSpdx, '-e', apacheGrant, '--', '.'],
    { cwd: fileURLToPath(rootFile('.')), encoding: 'utf8' },
  );
  assert.equal(
    apacheMarkers.status,
    1,
    `CyberMedica files still declare Apache-2.0:\n${apacheMarkers.stdout}${apacheMarkers.stderr}`,
  );

  assert.match(repositoryReadme, /Apache-2\.0 for EXOCHAIN core/);
  assert.match(repositoryReadme, /`livesafe\/` and `cybermedica\/`/);
  assert.match(repositoryReadme, /`livesafe\/LICENSE`/);
  assert.match(repositoryReadme, /`cybermedica\/LICENSE`/);
});
