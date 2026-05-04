import { redirect } from 'next/navigation';
import { getSession } from '@/lib/auth';
import { InternalShell } from '@/components/chrome/InternalShell';

export default function IntranetLayout({
  children
}: {
  children: React.ReactNode;
}) {
  const session = getSession();
  if (!session || session.surface !== 'intranet') {
    redirect('/internal/login');
  }
  return <InternalShell session={session}>{children}</InternalShell>;
}
