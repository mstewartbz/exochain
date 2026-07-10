// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import { Link } from 'react-router-dom';
import { ChevronRight } from 'lucide-react';
import { useInView } from './useInView';
import './landing.css';

const PACE_LETTERS = ['P', 'A', 'C', 'E'] as const;

export default function FinalCta() {
  const { ref, className } = useInView();

  return (
    <section
      className="py-24 md:py-28"
      style={{
        background:
          'radial-gradient(ellipse at 30% 0%, rgba(129,140,248,0.08), transparent 60%)',
      }}
    >
      <div className="max-w-6xl mx-auto px-6 md:px-8">
        {/* PACE letter tiles — border pulse staggered 1s apart, 8s loop.
            Static (dashed white/20) under reduced motion. */}
        <div
          ref={ref as React.RefObject<HTMLDivElement>}
          className={className}
        >
          <div className="grid grid-cols-4 gap-4 max-w-lg">
            {PACE_LETTERS.map((letter, i) => (
              <div
                key={letter}
                className="relative aspect-square rounded-2xl border-2 border-dashed border-white/20 flex items-center justify-center text-2xl font-heading font-bold text-white/40"
                style={{ '--i': i } as React.CSSProperties}
              >
                {/* Opacity-only glow pulse; animation lives in landing.css
                    under .ls-anim.play so it stays gated + reduced-motion safe. */}
                <span
                  aria-hidden="true"
                  className="ls-tile-pulse absolute -inset-0.5 rounded-2xl border-2 border-dashed border-blue-400/50 pointer-events-none"
                />
                {letter}
              </div>
            ))}
          </div>
          <p className="text-xs text-white/60 mt-4">
            Primary · Alternate · Contingency · Emergency
          </p>
        </div>

        <h2 className="text-4xl md:text-5xl font-heading font-extrabold text-white mt-12 mb-5 leading-tight">
          {/* Gradient budget slot 4 of 4: "you" — budget now spent */}
          Someone should be able to count on{' '}
          <span className="ls-grad-text">you</span>.
        </h2>
        <p className="text-lg text-gray-400 max-w-xl mb-10 leading-relaxed">
          LiveSafe is built around naming four trusted people — and being
          nameable by someone else. Naming your four is where it starts.
        </p>

        <div className="flex flex-col sm:flex-row items-start sm:items-center gap-4">
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
      </div>
    </section>
  );
}
