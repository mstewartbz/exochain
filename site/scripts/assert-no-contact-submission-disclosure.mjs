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

import { readFile } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const siteRoot = path.resolve(scriptDir, '..');
const supportPagePath = path.join(siteRoot, 'src/app/internal/support/page.tsx');
const contactRoutePath = path.join(siteRoot, 'src/app/api/contact/route.ts');
const contactSubmissionLibPath = path.join(siteRoot, 'src/lib/contact-submissions.ts');

function assertAbsent(source, pattern, message) {
  if (pattern.test(source)) {
    throw new Error(message);
  }
}

function assertPresent(source, pattern, message) {
  if (!pattern.test(source)) {
    throw new Error(message);
  }
}

const supportPage = await readFile(supportPagePath, 'utf8');
const contactRoute = await readFile(contactRoutePath, 'utf8');
const contactSubmissionLib = await readFile(contactSubmissionLibPath, 'utf8');

const forbiddenSupportPagePatterns = [
  {
    pattern: /contact-submissions/,
    message: 'internal support page must not import or reference the contact-submissions queue',
  },
  {
    pattern: /listRecentContactSubmissions/,
    message: 'internal support page must not query recent contact submissions',
  },
  {
    pattern: /\bContactSubmission\b/,
    message: 'internal support page must not type itself around contact-submission rows',
  },
  {
    pattern: /\bsubmittedAt\b|\bname\b|\bemail\b|\borganization\b|\brole\b|\bintendedUse\b|\buserAgent\b|\bforwardedFor\b|\bnotificationStatus\b|\bnotificationError\b/,
    message: 'internal support page must not render contact-submission sensitive fields',
  },
  {
    pattern: /Public contact submissions|No contact submissions are queued|Contact submission database/,
    message: 'internal support page must not expose contact-submission queue UI text',
  },
];

for (const { pattern, message } of forbiddenSupportPagePatterns) {
  assertAbsent(supportPage, pattern, message);
}

assertPresent(
  supportPage,
  /const TICKETS = \[/,
  'internal support page should remain a non-sensitive ticket queue view',
);
assertPresent(
  supportPage,
  /Support queue/,
  'internal support route should still render the support queue page',
);
assertPresent(
  contactRoute,
  /createContactSubmission/,
  'public contact route must keep persisting contact submissions',
);
assertPresent(
  contactRoute,
  /updateContactSubmissionNotification/,
  'public contact route must keep recording notification delivery state',
);
assertPresent(
  contactSubmissionLib,
  /\/api\/v1\/dag-db\/intake/,
  'contact-submission DAG DB backend must remain available',
);
assertAbsent(
  contactSubmissionLib,
  /site_contact_submissions|site_contact_rate_limits|CONTACT_DATABASE_URL/,
  'contact-submission backend must not expose legacy public-table storage',
);

console.log('Contact-submission disclosure guard passed.');
