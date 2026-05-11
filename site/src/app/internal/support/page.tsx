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

import { IntPageHead } from '@/components/content/IntPageHead';
import { Card, CardBody, CardHeader } from '@/components/ui/Card';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { Pill } from '@/components/ui/Pill';
import {
  listRecentContactSubmissions,
  type ContactSubmission,
} from '@/lib/contact-submissions';

export const metadata = { title: 'Support queue' };
export const dynamic = 'force-dynamic';

const COLUMNS: Column<ContactSubmission>[] = [
  {
    key: 'submittedAt',
    header: 'Submitted',
    width: '180px',
    render: (row) => <span className="font-mono text-xs">{row.submittedAt}</span>,
  },
  {
    key: 'name',
    header: 'Contact',
    width: '260px',
    render: (row) => (
      <div>
        <div className="font-medium">{row.name}</div>
        <div className="font-mono text-xs text-ink/60 dark:text-vellum-soft/60">{row.email}</div>
      </div>
    ),
  },
  {
    key: 'organization',
    header: 'Organization',
    width: '180px',
  },
  {
    key: 'intendedUse',
    header: 'Intended use',
    render: (row) => (
      <div>
        <div>{row.intendedUse || '—'}</div>
        <div className="mt-1 font-mono text-xs text-ink/50 dark:text-vellum-soft/50">
          {row.role || 'No role selected'}
        </div>
      </div>
    ),
  },
  {
    key: 'notificationStatus',
    header: 'Email',
    width: '140px',
    render: (row) => (
      <Pill tone={row.notificationStatus === 'sent' ? 'signal' : 'alert'}>
        {row.notificationStatus}
      </Pill>
    ),
  },
];

async function getSubmissions(): Promise<{
  rows: ContactSubmission[];
  error: string;
}> {
  try {
    return { rows: await listRecentContactSubmissions(), error: '' };
  } catch (error) {
    console.error('Unable to load contact submissions.', { error });
    return {
      rows: [],
      error: 'Contact submission database is not reachable.',
    };
  }
}

export default async function Page() {
  const { rows, error } = await getSubmissions();

  return (
    <>
      <IntPageHead
        eyebrow="Intranet · support"
        title="Support queue"
        lede="Recent public contact inquiries and support handoffs."
      />
      <Card>
        <CardHeader title="Public contact submissions" />
        <CardBody>
          {error ? (
            <div className="border border-amber-500/30 bg-amber-500/10 px-4 py-3 text-sm text-amber-700 dark:text-amber-300">
              {error}
            </div>
          ) : (
            <DataTable
              columns={COLUMNS}
              rows={rows}
              empty="No contact submissions are queued."
            />
          )}
        </CardBody>
      </Card>
    </>
  );
}
