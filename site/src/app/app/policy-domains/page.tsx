import { AppPageHead } from '@/components/content/AppPageHead';
import { DataTable, type Column } from '@/components/ui/DataTable';
import { mockPolicyDomains } from '@/lib/mock-data';
import type { PolicyDomain } from '@/lib/types';

export const metadata = { title: 'Policy domains' };

const cols: Column<PolicyDomain>[] = [
  { key: 'name', header: 'Domain' },
  { key: 'description', header: 'Description' },
  { key: 'ownerActorId', header: 'Owner', render: (r) => <span className="font-mono text-xs">{r.ownerActorId}</span> }
];

export default function Page() {
  return (
    <>
      <AppPageHead
        eyebrow="Extranet · policy"
        title="Policy domains"
        lede="Named action sets and constraints under which AVCs are valid."
      />
      <DataTable columns={cols} rows={mockPolicyDomains} />
    </>
  );
}
