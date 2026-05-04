import type { Metadata } from 'next';
import './globals.css';

export const metadata: Metadata = {
  title: {
    default: 'EXOCHAIN — Chain-of-custody for autonomous execution',
    template: '%s · EXOCHAIN'
  },
  description:
    'EXOCHAIN is chain-of-custody infrastructure for autonomous execution. Credential autonomous intent, verify delegated authority, and preserve evidentiary custody for agents, holons, humans, and AI-native systems.',
  metadataBase: new URL('https://exochain.io'),
  openGraph: {
    title: 'EXOCHAIN — Chain-of-custody for autonomous execution',
    description:
      'Custody-native blockchain. Credentialed volition. Evidentiary execution.',
    type: 'website'
  },
  robots: { index: true, follow: true }
};

export default function RootLayout({
  children
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="antialiased">{children}</body>
    </html>
  );
}
