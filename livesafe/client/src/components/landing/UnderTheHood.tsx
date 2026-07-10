// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import type { ReactNode } from 'react';
import {
  Fingerprint, KeyRound, TimerReset, Landmark, type LucideIcon,
} from 'lucide-react';
import {
  getLandingPublicTrustDisplayCopy,
  type LandingPublicTrustDisplayCopy,
  type PublicTrustRouteContract,
} from './publicTrustDisplayCopy';

interface HoodCard {
  icon: LucideIcon;
  title: string;
  body: ReactNode;
}

interface UnderTheHoodProps {
  trustStatus?: PublicTrustRouteContract | null;
}

/*
 * Copy hedge contract: "designed to / architecture calls for / in the
 * protocol" hedges are load-bearing legal-truth anchors. Do not strip.
 */
function buildCards(copy: LandingPublicTrustDisplayCopy): HoodCard[] {
  return [
    {
      icon: Fingerprint,
      title: 'Identity without accounts',
      body: (
        <>
          Your identifier is designed to be derived on your device from your
          passphrase into a{' '}
          <code className="font-mono text-xs text-gray-400">
            {copy.identityIdentifierLabel}
          </code>
          . There is no password table to breach because there are no passwords.
        </>
      ),
    },
    {
      icon: KeyRound,
      title: 'Your passphrase stays home',
      body: (
        <>
          The passphrase itself is designed never to be transmitted. Identity
          derivation belongs client-side, and the architecture calls for
          encryption envelopes around your profile.
        </>
      ),
    },
    {
      icon: TimerReset,
      title: 'Consent with a clock',
      body: (
        <>
          Every scan grant in the protocol carries{' '}
          <code className="font-mono text-xs text-gray-400">consent_expires_at_ms</code>.
          Access that can&rsquo;t expire is exposure, not consent.
        </>
      ),
    },
    {
      icon: Landmark,
      title: copy.governanceCardTitle,
      body: copy.governanceCardBody,
    },
  ];
}

export default function UnderTheHood({ trustStatus = null }: UnderTheHoodProps) {
  const copy = getLandingPublicTrustDisplayCopy(trustStatus);
  const cards = buildCards(copy);

  return (
    <section id="under-the-hood" className="scroll-mt-16 bg-white/[0.02] border-y border-white/[0.06] py-20 md:py-28">
      <div className="max-w-6xl mx-auto px-6 md:px-8">
        <p className="text-xs uppercase tracking-widest text-blue-400/70 font-heading font-semibold mb-4">
          ARCHITECTURE
        </p>
        <h2 className="text-3xl md:text-4xl font-heading font-bold text-white mb-5">
          {copy.underTheHoodHeading}
        </h2>
        <p className="text-gray-400 max-w-2xl leading-relaxed mb-12">
          {copy.underTheHoodBody}
        </p>

        <div className="grid md:grid-cols-2 gap-6">
          {cards.map(({ icon: Icon, title, body }) => (
            <div
              key={title}
              className="p-8 bg-white/[0.03] border border-white/10 rounded-2xl hover:border-blue-400/30 transition-colors"
            >
              <div className="w-11 h-11 rounded-xl bg-blue-500/10 text-blue-400 flex items-center justify-center mb-5">
                <Icon size={20} aria-hidden="true" />
              </div>
              <h3 className="text-lg font-heading font-semibold text-white mb-2">{title}</h3>
              <p className="text-sm text-gray-400 leading-relaxed">{body}</p>
            </div>
          ))}
        </div>

        <p className="text-xs text-gray-500 mt-10">
          {copy.footerStatus} · Apache-2.0 · open specification
        </p>
      </div>
    </section>
  );
}
