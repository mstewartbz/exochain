import { redirect } from 'next/navigation';
import { getSession } from '@/lib/auth';
import { AppShell } from '@/components/chrome/AppShell';

export default function ExtranetLayout({
  children
}: {
  children: React.ReactNode;
}) {
  const session = getSession();
  if (!session || session.surface !== 'extranet') {
    redirect('/app/login');
  }
  return <AppShell session={session}>{children}</AppShell>;
}
