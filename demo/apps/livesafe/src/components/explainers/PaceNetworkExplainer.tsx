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

// PaceNetworkExplainer — "Threshold custody" animated inline-SVG explainer.
//
// Self-contained, zero props. One 14s CSS-keyframe loop, gated by an
// IntersectionObserver (threshold 0.35, fires once). Every element's CSS
// rest state IS the complete static diagram, so prefers-reduced-motion
// (which kills all animation via the media query below) degrades to a
// full, legible figure: shards docked with locks, P/A/C highlighted with
// the dashed triangle, "ANY 3 OF 4" label, meter at 3/3 with the
// "Unilateral access: 0/4" subcaption. Elements that exist only for
// motion carry .lspn-mo (opacity: 0 at rest) and never appear without
// animation. No SMIL, no JS animation — CSS keyframes only.

import { useEffect, useRef, useState } from 'react';

const SORA = "'Sora', system-ui, sans-serif";

/** PACE role geometry + semantic colors (P blue / A cyan / C amber / E red). */
const NODES = [
  { k: 'p', letter: 'P', role: 'Primary', color: '#3b82f6', x: 260, y: 70, labelY: -42 },
  { k: 'a', letter: 'A', role: 'Alternate', color: '#06b6d4', x: 410, y: 192, labelY: 52 },
  { k: 'c', letter: 'C', role: 'Contingency', color: '#f59e0b', x: 260, y: 314, labelY: 52 },
  { k: 'e', letter: 'E', role: 'Emergency', color: '#ef4444', x: 110, y: 192, labelY: 52 },
] as const;

/** Spokes drawn center → trustee (x1/y1 at the center side so the draw radiates outward). */
const SPOKES = [
  { k: 'p', x1: 260, y1: 150, x2: 260, y2: 98 },
  { k: 'a', x1: 302, y1: 192, x2: 382, y2: 192 },
  { k: 'c', x1: 260, y1: 234, x2: 260, y2: 286 },
  { k: 'e', x1: 218, y1: 192, x2: 138, y2: 192 },
] as const;

/**
 * All choreography lives in one 14s timeline; percentages below are seconds/14.
 * 0.0–0.8 center fades up holding the whole key · 1.0–3.4 trustees pop P→A→C→E
 * with spokes drawing behind · 3.8–4.6 the key fractures into four shards ·
 * 4.8–6.6 shards travel the spokes and dock with lock badges · 7.0–8.2 the
 * P/A/C trio + triangle light up, meter ticks 1/3→2/3→3/3 ✓ · 8.2–9.2 shards
 * echo-pulse home and a ghost key assembles · 9.4–10.4 counter-case: E alone
 * pulses, its return line stops halfway, meter shows 1/3 ✗ · 10.6–11.4 the
 * highlight rotates to A/C/E (any three works) · 11.4–13.2 hold · fade, loop.
 */
const CSS = `
.lspn-fig{margin:0}
.lspn .lspn-mo{opacity:0}
.lspn .lspn-tb{transform-box:fill-box;transform-origin:50% 50%}

@keyframes lspn-center{0%{opacity:0;transform:translateY(12px)}5.7%{opacity:1;transform:translateY(0)}100%{opacity:1;transform:translateY(0)}}
@keyframes lspn-cpulse{0%,27.1%{transform:scale(1)}30%{transform:scale(1.03)}32.9%,100%{transform:scale(1)}}
@keyframes lspn-wholekey{0%{opacity:0}5.7%{opacity:1}27.1%{opacity:1}32.9%,100%{opacity:0}}

@keyframes lspn-node-p{0%,7.1%{opacity:0;transform:scale(.6);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}11%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}
@keyframes lspn-node-a{0%,11.4%{opacity:0;transform:scale(.6);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}15.3%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}
@keyframes lspn-node-c{0%,15.7%{opacity:0;transform:scale(.6);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}19.6%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}
@keyframes lspn-node-e{0%,20%{opacity:0;transform:scale(.6);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}23.9%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}

@keyframes lspn-spoke-p{0%,8.6%{stroke-dashoffset:1;opacity:1}12.1%{stroke-dashoffset:0;opacity:1}81.4%{opacity:1}86%{opacity:.55}90.7%{opacity:1}94.3%{opacity:1}100%{opacity:0}}
@keyframes lspn-spoke-a{0%,12.9%{stroke-dashoffset:1;opacity:1}16.4%{stroke-dashoffset:0;opacity:1}81.4%{opacity:1}86%{opacity:.55}90.7%{opacity:1}94.3%{opacity:1}100%{opacity:0}}
@keyframes lspn-spoke-c{0%,17.1%{stroke-dashoffset:1;opacity:1}20.7%{stroke-dashoffset:0;opacity:1}81.4%{opacity:1}86%{opacity:.55}90.7%{opacity:1}94.3%{opacity:1}100%{opacity:0}}
@keyframes lspn-spoke-e{0%,21.4%{stroke-dashoffset:1;opacity:1}25%{stroke-dashoffset:0;opacity:1}81.4%{opacity:1}86%{opacity:.55}90.7%{opacity:1}94.3%{opacity:1}100%{opacity:0}}

@keyframes lspn-tshard-p{0%,27.1%{opacity:0;transform:translate(0,0)}32.9%{opacity:1;transform:translate(0,-9px)}34.3%{opacity:1;transform:translate(0,-9px)}37.5%{opacity:1;transform:translate(0,-122px)}39%,100%{opacity:0;transform:translate(0,-122px)}}
@keyframes lspn-tshard-a{0%,27.1%{opacity:0;transform:translate(0,0)}32.9%{opacity:1;transform:translate(9px,0)}37.5%{opacity:1;transform:translate(9px,0)}40.7%{opacity:1;transform:translate(150px,0)}42.1%,100%{opacity:0;transform:translate(150px,0)}}
@keyframes lspn-tshard-c{0%,27.1%{opacity:0;transform:translate(0,0)}32.9%{opacity:1;transform:translate(0,9px)}40.7%{opacity:1;transform:translate(0,9px)}43.9%{opacity:1;transform:translate(0,122px)}45.4%,100%{opacity:0;transform:translate(0,122px)}}
@keyframes lspn-tshard-e{0%,27.1%{opacity:0;transform:translate(0,0)}32.9%{opacity:1;transform:translate(-9px,0)}43.9%{opacity:1;transform:translate(-9px,0)}47.1%{opacity:1;transform:translate(-150px,0)}48.6%,100%{opacity:0;transform:translate(-150px,0)}}

@keyframes lspn-dock-p{0%,37.5%{opacity:0;transform:scale(.5);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}39.5%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}
@keyframes lspn-dock-a{0%,40.7%{opacity:0;transform:scale(.5);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}42.7%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}
@keyframes lspn-dock-c{0%,43.9%{opacity:0;transform:scale(.5);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}45.9%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}
@keyframes lspn-dock-e{0%,47.1%{opacity:0;transform:scale(.5);animation-timing-function:cubic-bezier(.34,1.56,.64,1)}49.1%{opacity:1;transform:scale(1)}94.3%{opacity:1;transform:scale(1)}100%{opacity:0;transform:scale(1)}}

@keyframes lspn-shamir{0%,27.1%{opacity:0}30%{opacity:1}50%{opacity:1}53%,100%{opacity:0}}
@keyframes lspn-hl-pac{0%,50%{opacity:0}55.7%{opacity:1}65.7%{opacity:1}67.1%{opacity:0}81.4%{opacity:0}83.6%{opacity:1}94.3%{opacity:1}100%{opacity:0}}
@keyframes lspn-hl-ace{0%,75.7%{opacity:0}77.1%{opacity:1}80%{opacity:1}82.1%,100%{opacity:0}}
@keyframes lspn-any3{0%,52.9%{opacity:0}54.5%{opacity:1}65.7%{opacity:1}67.1%{opacity:0}75.7%{opacity:0}77.1%{opacity:1}94.3%{opacity:1}100%{opacity:0}}

@keyframes lspn-m13{0%,50.7%{opacity:0}51.4%{opacity:1}52.9%,100%{opacity:0}}
@keyframes lspn-m23{0%,52.9%{opacity:0}53.6%{opacity:1}55.7%,100%{opacity:0}}
@keyframes lspn-m33{0%,55.7%{opacity:0}57.1%{opacity:1}65.7%{opacity:1}67.1%{opacity:0}74.3%{opacity:0}75.7%{opacity:1}94.3%{opacity:1}100%{opacity:0}}
@keyframes lspn-mfail{0%,67.1%{opacity:0}68.6%{opacity:.7}72.9%{opacity:.7}74.3%,100%{opacity:0}}

@keyframes lspn-ghost{0%,58.6%{opacity:0}61.4%{opacity:.5}65.7%,100%{opacity:0}}
@keyframes lspn-echo-p{0%,58.6%{opacity:0;transform:translate(0,-122px)}59.6%{opacity:.8}62.9%,100%{opacity:0;transform:translate(0,0)}}
@keyframes lspn-echo-a{0%,59.3%{opacity:0;transform:translate(150px,0)}60.3%{opacity:.8}63.6%,100%{opacity:0;transform:translate(0,0)}}
@keyframes lspn-echo-c{0%,60%{opacity:0;transform:translate(0,122px)}61%{opacity:.8}64.3%,100%{opacity:0;transform:translate(0,0)}}

@keyframes lspn-epulse{0%,67.1%{opacity:0}68.6%{opacity:.8}70%{opacity:.25}71.4%{opacity:.8}72.9%{opacity:.25}74.3%,100%{opacity:0}}
@keyframes lspn-ereturn{0%,67.1%{opacity:0;stroke-dashoffset:1}67.9%{opacity:.9;stroke-dashoffset:.86}70.7%{opacity:.9;stroke-dashoffset:.5}72.9%{opacity:.9;stroke-dashoffset:.5}74.3%,100%{opacity:0;stroke-dashoffset:.5}}

.lspn.play .k-center{animation:lspn-center 14s ease-out infinite both}
.lspn.play .k-cpulse{animation:lspn-cpulse 14s ease-in-out infinite both}
.lspn.play .k-wholekey{animation:lspn-wholekey 14s ease-out infinite both}
.lspn.play .k-node-p{animation:lspn-node-p 14s ease-out infinite both}
.lspn.play .k-node-a{animation:lspn-node-a 14s ease-out infinite both}
.lspn.play .k-node-c{animation:lspn-node-c 14s ease-out infinite both}
.lspn.play .k-node-e{animation:lspn-node-e 14s ease-out infinite both}
.lspn.play .k-spoke-p{animation:lspn-spoke-p 14s ease-out infinite both}
.lspn.play .k-spoke-a{animation:lspn-spoke-a 14s ease-out infinite both}
.lspn.play .k-spoke-c{animation:lspn-spoke-c 14s ease-out infinite both}
.lspn.play .k-spoke-e{animation:lspn-spoke-e 14s ease-out infinite both}
.lspn.play .k-tshard-p{animation:lspn-tshard-p 14s ease-in-out infinite both}
.lspn.play .k-tshard-a{animation:lspn-tshard-a 14s ease-in-out infinite both}
.lspn.play .k-tshard-c{animation:lspn-tshard-c 14s ease-in-out infinite both}
.lspn.play .k-tshard-e{animation:lspn-tshard-e 14s ease-in-out infinite both}
.lspn.play .k-dock-p{animation:lspn-dock-p 14s ease-out infinite both}
.lspn.play .k-dock-a{animation:lspn-dock-a 14s ease-out infinite both}
.lspn.play .k-dock-c{animation:lspn-dock-c 14s ease-out infinite both}
.lspn.play .k-dock-e{animation:lspn-dock-e 14s ease-out infinite both}
.lspn.play .k-shamir{animation:lspn-shamir 14s ease-out infinite both}
.lspn.play .k-hl-pac{animation:lspn-hl-pac 14s ease-out infinite both}
.lspn.play .k-hl-ace{animation:lspn-hl-ace 14s ease-out infinite both}
.lspn.play .k-any3{animation:lspn-any3 14s ease-out infinite both}
.lspn.play .k-m13{animation:lspn-m13 14s linear infinite both}
.lspn.play .k-m23{animation:lspn-m23 14s linear infinite both}
.lspn.play .k-m33{animation:lspn-m33 14s linear infinite both}
.lspn.play .k-mfail{animation:lspn-mfail 14s linear infinite both}
.lspn.play .k-ghost{animation:lspn-ghost 14s ease-in-out infinite both}
.lspn.play .k-echo-p{animation:lspn-echo-p 14s ease-in infinite both}
.lspn.play .k-echo-a{animation:lspn-echo-a 14s ease-in infinite both}
.lspn.play .k-echo-c{animation:lspn-echo-c 14s ease-in infinite both}
.lspn.play .k-epulse{animation:lspn-epulse 14s ease-in-out infinite both}
.lspn.play .k-ereturn{animation:lspn-ereturn 14s ease-out infinite both}

@media (prefers-reduced-motion: reduce){
  .lspn, .lspn *{animation:none !important;transition:none !important}
}
`;

/** Fires once when the figure is ≥35% visible, then unobserves. */
function useInViewOnce() {
  const ref = useRef<HTMLElement | null>(null);
  const [inView, setInView] = useState(false);
  useEffect(() => {
    const el = ref.current;
    if (!el) return;
    if (typeof IntersectionObserver === 'undefined') {
      setInView(true);
      return;
    }
    const obs = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setInView(true);
            obs.disconnect();
          }
        }
      },
      { threshold: 0.35 },
    );
    obs.observe(el);
    return () => obs.disconnect();
  }, []);
  return { ref, inView };
}

/** Abstract key-fragment quarter — deliberately not real key geometry. */
function ShardGlyph({ color }: { color: string }) {
  return <polygon points="-6,-2 -1,-4.5 5.5,-1.5 3,3.5 -3.5,4.5" fill={color} fillOpacity="0.85" />;
}

/** Tiny lock badge (docked-shard custody marker). */
function LockBadge() {
  return (
    <g>
      <path d="M -2.2 -1.2 v -1.8 a 2.2 2.2 0 0 1 4.4 0 v 1.8" fill="none" stroke="#9ca3af" strokeWidth="1.1" />
      <rect x="-3.4" y="-1.2" width="6.8" height="5.6" rx="1.2" fill="#101a2e" stroke="#9ca3af" strokeWidth="1.1" />
    </g>
  );
}

/** Simple key glyph — bow, shaft, two teeth. */
function KeyGlyph({ stroke }: { stroke: string }) {
  return (
    <g fill="none" stroke={stroke} strokeWidth="2" strokeLinecap="round">
      <circle cx="-10" cy="0" r="4.5" />
      <path d="M -5.5 0 H 11 M 5 0 V 5 M 9.5 0 V 4" />
    </g>
  );
}

const CAPTION =
  'Your key splits four ways by design. Any three trustees, together, can act. No one can alone.';

export function PaceNetworkExplainer() {
  const { ref, inView } = useInViewOnce();

  return (
    <figure
      ref={ref as React.RefObject<HTMLElement>}
      className={`lspn lspn-fig ls-anim${inView ? ' play' : ''}`}
      role="img"
      aria-label={`PACE threshold custody diagram. ${CAPTION}`}
    >
      <style>{CSS}</style>
      <svg viewBox="0 0 520 420" className="w-full h-auto" aria-hidden="true" xmlns="http://www.w3.org/2000/svg">
        {/* ── Dashed threshold triangles (behind nodes) ─────────────────── */}
        {/* P/A/C triangle — the canonical "any 3 of 4" trio; rest-visible */}
        <path
          className="k-hl-pac"
          d="M 286.4 91.4 L 383.6 170.6 M 383.6 213.4 L 286.4 292.6 M 260 280 L 260 104"
          fill="none"
          stroke="rgba(96,165,250,0.5)"
          strokeWidth="1.5"
          strokeDasharray="5 7"
        />
        {/* A/C/E rotation — motion-only (order doesn't matter beat) */}
        <path
          className="lspn-mo k-hl-ace"
          d="M 383.6 213.4 L 286.4 292.6 M 233.6 292.6 L 136.4 213.4 M 144 192 L 376 192"
          fill="none"
          stroke="rgba(34,211,238,0.5)"
          strokeWidth="1.5"
          strokeDasharray="5 7"
        />

        {/* ── Spokes center → trustees ──────────────────────────────────── */}
        {SPOKES.map((s) => (
          <line
            key={s.k}
            className={`k-spoke-${s.k}`}
            x1={s.x1}
            y1={s.y1}
            x2={s.x2}
            y2={s.y2}
            pathLength={1}
            stroke="rgba(255,255,255,0.14)"
            strokeWidth="1.5"
            style={{ strokeDasharray: 1 }}
          />
        ))}

        {/* Counter-case: E's lone return line draws halfway and stops */}
        <line
          className="lspn-mo k-ereturn"
          x1={140}
          y1={184}
          x2={216}
          y2={184}
          pathLength={1}
          stroke="#ef4444"
          strokeWidth="1.5"
          style={{ strokeDasharray: 1 }}
        />

        {/* ── Threshold highlight rings (P/A/C rest-visible, A/C/E motion) ─ */}
        {(['p', 'a', 'c'] as const).map((k) => {
          const n = NODES.find((nd) => nd.k === k)!;
          return (
            <circle
              key={`pac-${k}`}
              className="k-hl-pac"
              cx={n.x}
              cy={n.y}
              r={36}
              fill="none"
              stroke="#60a5fa"
              strokeWidth="2"
            />
          );
        })}
        {(['a', 'c', 'e'] as const).map((k) => {
          const n = NODES.find((nd) => nd.k === k)!;
          return (
            <circle
              key={`ace-${k}`}
              className="lspn-mo k-hl-ace"
              cx={n.x}
              cy={n.y}
              r={36}
              fill="none"
              stroke="#22d3ee"
              strokeWidth="2"
            />
          );
        })}

        {/* Counter-case halo: E alone can't reach threshold */}
        <circle className="lspn-mo k-epulse" cx={110} cy={192} r={36} fill="none" stroke="#ef4444" strokeWidth="2" />

        {/* ── Trustee nodes ─────────────────────────────────────────────── */}
        {NODES.map((n) => (
          <g key={n.k} transform={`translate(${n.x} ${n.y})`}>
            <g className={`lspn-tb k-node-${n.k}`}>
              <circle r={32} fill="none" stroke={n.color} strokeWidth="1.5" opacity="0.55" />
              <circle r={28} fill="rgba(255,255,255,0.04)" stroke="rgba(255,255,255,0.16)" strokeWidth="1.5" />
              <text
                y={1}
                textAnchor="middle"
                dominantBaseline="middle"
                fontFamily={SORA}
                fontSize="15"
                fontWeight="600"
                fill="#e5e7eb"
              >
                {n.letter}
              </text>
              <text
                y={n.labelY}
                textAnchor="middle"
                fontFamily={SORA}
                fontSize="10"
                fill="#9ca3af"
              >
                {n.role}
              </text>
            </g>
            {/* Docked shard + lock badge — arrive when the traveling shard lands */}
            <g className={`lspn-tb k-dock-${n.k}`}>
              <g transform="translate(0 13)">
                <ShardGlyph color={n.color} />
              </g>
              <g transform="translate(20 -20)">
                <LockBadge />
              </g>
            </g>
          </g>
        ))}

        {/* ── Traveling shards (fracture → distribution; motion-only) ───── */}
        <g transform="translate(260 192)">
          {NODES.map((n) => (
            <g key={`t-${n.k}`} className={`lspn-mo k-tshard-${n.k}`}>
              <ShardGlyph color={n.color} />
            </g>
          ))}
        </g>

        {/* ── Echo pulses: the lit trio's shards flow home (motion-only) ── */}
        <g transform="translate(260 192)">
          <circle className="lspn-mo k-echo-p" r={3} fill="#60a5fa" />
          <circle className="lspn-mo k-echo-a" r={3} fill="#60a5fa" />
          <circle className="lspn-mo k-echo-c" r={3} fill="#60a5fa" />
        </g>

        {/* ── Center: YOU (shield only at rest — no whole key to steal) ─── */}
        <g transform="translate(260 192)">
          <g className="k-center">
            <g className="lspn-tb k-cpulse">
              <circle r={42} fill="rgba(255,255,255,0.03)" stroke="rgba(255,255,255,0.16)" strokeWidth="1.5" />
              <path
                d="M 0 -31 l 10 3.8 v 7.2 c 0 6.5 -4 10.7 -10 13.5 c -6 -2.8 -10 -7 -10 -13.5 v -7.2 z"
                fill="none"
                stroke="#60a5fa"
                strokeWidth="1.8"
                strokeLinejoin="round"
              />
              <text
                y={26}
                textAnchor="middle"
                fontFamily={SORA}
                fontSize="11"
                fontWeight="600"
                letterSpacing="2"
                fill="#e5e7eb"
              >
                YOU
              </text>
            </g>
            {/* Whole key — exists only before the Shamir fracture */}
            <g className="lspn-mo k-wholekey" transform="translate(0 8)">
              <KeyGlyph stroke="#e5e7eb" />
            </g>
            {/* Ghost key — assembles when three shards agree, never solidifies */}
            <g className="lspn-mo k-ghost" transform="translate(0 8)">
              <KeyGlyph stroke="#22d3ee" />
            </g>
          </g>
        </g>

        {/* ── Labels ────────────────────────────────────────────────────── */}
        <text
          className="lspn-mo k-shamir"
          x={260}
          y={254}
          textAnchor="middle"
          fontFamily={SORA}
          fontSize="11"
          fill="#9ca3af"
        >
          Shamir split · spec
        </text>
        <text
          className="k-any3"
          x={392}
          y={108}
          textAnchor="middle"
          fontFamily={SORA}
          fontSize="11"
          letterSpacing="1.5"
          fill="#60a5fa"
        >
          ANY 3 OF 4
        </text>

        {/* ── Threshold meter ───────────────────────────────────────────── */}
        <g>
          <rect x={218} y={377} width={84} height={26} rx={13} fill="rgba(255,255,255,0.03)" stroke="rgba(255,255,255,0.14)" />
          <text
            className="lspn-mo k-m13"
            x={260}
            y={394}
            textAnchor="middle"
            fontFamily={SORA}
            fontSize="12"
            fontWeight="600"
            fill="#9ca3af"
          >
            1/3
          </text>
          <text
            className="lspn-mo k-m23"
            x={260}
            y={394}
            textAnchor="middle"
            fontFamily={SORA}
            fontSize="12"
            fontWeight="600"
            fill="#9ca3af"
          >
            2/3
          </text>
          <text
            className="k-m33"
            x={260}
            y={394}
            textAnchor="middle"
            fontFamily={SORA}
            fontSize="12"
            fontWeight="600"
            fill="#60a5fa"
          >
            3/3 ✓
          </text>
          <text
            className="lspn-mo k-mfail"
            x={260}
            y={394}
            textAnchor="middle"
            fontFamily={SORA}
            fontSize="12"
            fontWeight="600"
            fill="#ef4444"
          >
            1/3 ✗
          </text>
          <text x={314} y={394} fontFamily={SORA} fontSize="10" fill="rgba(255,255,255,0.4)">
            Unilateral access: 0/4
          </text>
        </g>
      </svg>
      <figcaption className="mt-3 text-xs text-white/60">{CAPTION}</figcaption>
    </figure>
  );
}

export default PaceNetworkExplainer;
