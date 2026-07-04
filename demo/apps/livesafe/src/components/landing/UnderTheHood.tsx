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

import {
  Fingerprint, KeyRound, TimerReset, Landmark, type LucideIcon,
} from 'lucide-react';

interface HoodCard {
  icon: LucideIcon;
  title: string;
  body: React.ReactNode;
}

/*
 * Copy hedge contract: "designed to / architecture calls for / in the
 * protocol" hedges are load-bearing legal-truth anchors. Do not strip.
 */
const CARDS: HoodCard[] = [
  {
    icon: Fingerprint,
    title: 'Identity without accounts',
    body: (
      <>
        Your identifier is designed to be derived on your device from your passphrase —
        SHA-256 to a <code className="font-mono text-xs text-gray-400">did:exo</code>{' '}
        DID. There is no password table to breach because there are no passwords.
      </>
    ),
  },
  {
    icon: KeyRound,
    title: 'Your passphrase stays home',
    body: (
      <>
        The passphrase itself is designed never to be transmitted — identity
        derivation belongs client-side, and the architecture calls for X25519
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
        Access that can&rsquo;t expire isn&rsquo;t consent — it&rsquo;s exposure.
      </>
    ),
  },
  {
    icon: Landmark,
    title: 'Governed, not just hosted',
    body: (
      <>
        EXOCHAIN&rsquo;s constitutional model separates powers: no component is
        designed to both hold your data and decide who sees it. Governance is
        the runtime, not the PDF.
      </>
    ),
  },
];

export default function UnderTheHood() {
  return (
    <section id="under-the-hood" className="scroll-mt-16 bg-white/[0.02] border-y border-white/[0.06] py-20 md:py-28">
      <div className="max-w-6xl mx-auto px-6 md:px-8">
        <p className="text-xs uppercase tracking-widest text-blue-400/70 font-heading font-semibold mb-4">
          ARCHITECTURE
        </p>
        <h2 className="text-3xl md:text-4xl font-heading font-bold text-white mb-5">
          Built on a trust fabric, not a terms-of-service.
        </h2>
        <p className="text-gray-400 max-w-2xl leading-relaxed mb-12">
          LiveSafe is a demonstration app built for EXOCHAIN, a constitutional
          governance runtime. The
          rules that protect you — consent gating, identified access, threshold
          custody — are expressed as architecture, designed to be enforced by
          the system rather than promised by a policy page.
        </p>

        <div className="grid md:grid-cols-2 gap-6">
          {CARDS.map(({ icon: Icon, title, body }) => (
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
          EXOCHAIN v0.1.0-beta · Apache-2.0 · open specification
        </p>
      </div>
    </section>
  );
}
