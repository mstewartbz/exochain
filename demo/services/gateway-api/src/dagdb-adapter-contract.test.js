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

import { describe, expect, it } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const TEST_DIR = dirname(fileURLToPath(import.meta.url));
const DEMO_ROOT = resolve(TEST_DIR, '../../..');
const SERVICES_ROOT = join(DEMO_ROOT, 'services');
const SHARED_SOURCE = join(DEMO_ROOT, 'packages/shared/src/index.js');
const FIXTURE_NOTICE = join(DEMO_ROOT, 'infra/postgres/init/README.md');

function serviceIndexFiles() {
  return readdirSync(SERVICES_ROOT)
    .map((service) => join(SERVICES_ROOT, service, 'src/index.js'))
    .sort();
}

function read(path) {
  return readFileSync(path, 'utf8');
}

describe('demo services DAG DB adapter contract', () => {
  it('routes production persistence through the shared DAG DB store', () => {
    for (const file of serviceIndexFiles()) {
      const source = read(file);
      expect(source, `${file} must not import pg directly`).not.toMatch(/from ['"]pg['"]/);
      expect(source, `${file} must not construct pg.Pool directly`).not.toMatch(/new\s+pg\.Pool\s*\(/);
      expect(source, `${file} must not read DATABASE_URL directly`).not.toContain('DATABASE_URL');
      expect(source, `${file} must use createDemoServiceStore`).toContain('createDemoServiceStore');
    }

    const shared = read(SHARED_SOURCE);
    expect(shared, 'shared package must not expose a direct Postgres fallback').not.toMatch(/postgres:\/\/exochain/);
    expect(shared, 'shared package must export the DAG DB store factory').toContain('createDemoServiceStore');
  });

  it('keeps legacy demo SQL as an explicit fixture only', () => {
    const notice = read(FIXTURE_NOTICE);
    expect(notice).toContain('fixture-only');
    expect(notice).toContain('must not be mounted as a production writer');
  });
});
