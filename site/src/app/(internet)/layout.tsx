import { PublicNav } from '@/components/chrome/PublicNav';
import { PublicFooter } from '@/components/chrome/PublicFooter';

export default function InternetLayout({
  children
}: {
  children: React.ReactNode;
}) {
  return (
    <div className="min-h-dvh flex flex-col">
      <PublicNav />
      <div className="flex-1">{children}</div>
      <PublicFooter />
    </div>
  );
}
