// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import { Shield, QrCode, Users, Clock, type LucideIcon } from 'lucide-react';

interface Feature {
  icon: LucideIcon;
  title: string;
  body: string;
}

/** Demo-today features only — present tense, no capability overclaims. */
const FEATURES: Feature[] = [
  {
    icon: Shield,
    title: 'Emergency plans.',
    body: 'Draft response plans per scenario — rally points, go-bag checklists, communication plans, and evacuation routes.',
  },
  {
    icon: QrCode,
    title: 'Digital ICE card.',
    body: 'Enter your medical essentials and preview your card, with consent-gated responder access by design.',
  },
  {
    icon: Users,
    title: 'PACE trustee slots.',
    body: 'Four named roles — Primary, Alternate, Contingency, Emergency — each slot naming one person you trust.',
  },
  {
    icon: Clock,
    title: 'Golden hour checklists.',
    body: 'Tap-to-check protocols for three scenarios, with timeframe targets and critical steps flagged.',
  },
];

export default function FeaturesGrid() {
  return (
    <section className="py-20 md:py-28">
      <div className="max-w-6xl mx-auto px-6 md:px-8">
        <h2 className="text-3xl md:text-4xl font-heading font-bold text-white mb-12">
          Built for the day you hope never comes.
        </h2>
        <div className="grid md:grid-cols-2 gap-6">
          {FEATURES.map(({ icon: Icon, title, body }) => (
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
      </div>
    </section>
  );
}
