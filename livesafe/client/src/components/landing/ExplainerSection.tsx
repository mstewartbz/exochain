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

import { Link } from 'react-router-dom';
import { cn } from '../../lib/utils';
import './landing.css';

export interface ExplainerSectionProps {
  id?: string;
  eyebrow: string;
  title: string;
  body: React.ReactNode;
  triad: [string, string, string];
  /** Mono spec callout grounding the claim in the actual protocol. */
  spec?: { code: string; caption: string };
  microCta: { label: string; to: '/register' };
  /** true = figure on the left (desktop). */
  flip?: boolean;
  figure: React.ReactNode;
}

/**
 * Shared wrapper for the three landing explainers: copy column + animated
 * SVG figure, alternating sides. `content-visibility: auto` via `.ls-cv`.
 */
export default function ExplainerSection({
  id,
  eyebrow,
  title,
  body,
  triad,
  spec,
  microCta,
  flip = false,
  figure,
}: ExplainerSectionProps) {
  return (
    <section id={id} className="ls-cv scroll-mt-16 py-20 md:py-28">
      <div className="max-w-6xl mx-auto px-6 md:px-8">
        <div className="grid md:grid-cols-2 gap-12 lg:gap-16 items-center">
          <div className={cn(flip && 'md:order-2')}>
            <p className="text-xs uppercase tracking-widest text-blue-400/70 font-heading font-semibold mb-4">
              {eyebrow}
            </p>
            <h2 className="text-3xl md:text-4xl font-heading font-bold text-white mb-5">
              {title}
            </h2>
            <div className="text-gray-400 leading-relaxed mb-5">{body}</div>
            <p className="text-sm text-[#9ca3af] mb-6">{triad.join(' · ')}</p>
            {spec && (
              <div className="bg-white/[0.03] border border-white/10 rounded-lg p-4 mb-6">
                <pre className="font-mono text-xs text-gray-500 whitespace-pre-wrap overflow-x-auto">
                  {spec.code}
                </pre>
                <p className="text-xs text-white/60 mt-2">{spec.caption}</p>
              </div>
            )}
            <Link
              to={microCta.to}
              className="text-sm font-medium text-blue-300 hover:text-cyan-300 transition-colors"
            >
              {microCta.label}
            </Link>
          </div>
          <div className={cn(flip && 'md:order-1')}>{figure}</div>
        </div>
      </div>
    </section>
  );
}
