// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import { Link } from 'react-router-dom';
import { Shield, ChevronRight } from 'lucide-react';
import IceCardArt from './IceCardArt';
import './landing.css';

export default function Hero() {
  return (
    <section id="top" className="max-w-6xl mx-auto px-6 md:px-8 pt-20 md:pt-24 pb-20">
      <div className="grid lg:grid-cols-[1.1fr_0.9fr] gap-16 items-center">
        {/* Copy column */}
        <div>
          <div className="inline-flex items-center gap-2 bg-blue-500/10 border border-blue-500/20 rounded-full text-xs text-blue-300 px-3 py-1 mb-8">
            <Shield size={14} aria-hidden="true" />
            <span>Civil defense preparedness · demo preview</span>
          </div>

          <h1 className="text-5xl md:text-6xl leading-[1.05] font-heading font-extrabold text-white mb-6">
            Be someone&rsquo;s
            <br />
            {/* Gradient budget slot 2 of 4 */}
            <span className="ls-grad-text">safety net.</span>
          </h1>

          <p className="text-lg text-gray-400 max-w-xl mb-6 leading-relaxed">
            Build emergency plans for the scenarios that matter, name your four
            most trusted people, and build a digital ICE card designed to speak
            for you when you can&rsquo;t.
          </p>

          <p className="text-sm text-[#9ca3af] mb-8">
            ICE cards · PACE trust networks · Golden hour checklists
          </p>

          <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
            {/* Gradient budget slot 3 of 4: primary CTA (blue→cyan half) */}
            <Link
              to="/register"
              className="inline-flex items-center gap-2 bg-gradient-to-r from-blue-500 to-cyan-500 rounded-xl px-8 py-4 text-white font-heading font-semibold shadow-lg shadow-blue-500/25 hover:opacity-95 transition-opacity"
            >
              Create your safety net
              <ChevronRight size={18} aria-hidden="true" />
            </Link>
            <Link
              to="/login"
              className="inline-flex items-center border border-blue-400/30 hover:border-blue-400/60 text-blue-300 rounded-xl px-8 py-4 font-medium transition-colors"
            >
              I was invited by someone
            </Link>
          </div>

          <p className="text-xs text-gray-500 mt-4">
            Your passphrase is designed never to leave your device — identity
            is derived from it locally.
          </p>
        </div>

        {/* Card column — the page's single high-chroma object above the fold */}
        <div className="w-full max-w-md lg:max-w-none mx-auto lg:mx-0">
          <IceCardArt />
          <p className="text-xs text-white/60 text-center mt-4">
            Your info. Your consent. Their sixty seconds.
          </p>
        </div>
      </div>
    </section>
  );
}
