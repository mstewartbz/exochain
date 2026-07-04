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

/**
 * GoldenHourExplainer — "Sixty minutes, decided."
 *
 * Self-contained animated inline-SVG explainer for the LiveSafe landing page
 * (spec Part 4.3). A vertical 0–60 min timeline spine, four condensed steps
 * from the real medical protocol (GoldenHour.tsx DEFAULT_PROTOCOLS), checks
 * that land paced proportionally to their timeframes, a scaleX progress bar,
 * and a stacked-numeral counter. 13s CSS-keyframe loop, zero JS animation.
 *
 * Final-frame doctrine: every element's resting SVG attributes ARE the
 * completed diagram. Keyframes only supply the hidden/offset `from` states,
 * so under `prefers-reduced-motion: reduce` (animations killed wholesale)
 * the figure degrades to a complete static sample protocol. Elements that
 * exist only for motion (`.lsgh-motion-only`) stay hidden.
 *
 * Animation gating: an IntersectionObserver (threshold 0.35) adds the play
 * class once, then unobserves. Loops are infinite so a late start is
 * harmless. All loop choreography is expressed as percentages of the single
 * 13s timeline — no animation-delay on looped elements — so the cycle can
 * never drift out of sync.
 */

import { useEffect, useRef, useState } from 'react';

/* ------------------------------------------------------------------ */
/* Choreography (seconds within the 13s loop; spec Part 4.3 timeline)  */
/* ------------------------------------------------------------------ */

const LOOP_S = 13;

/** Seconds → percentage of the 13s loop, e.g. p(4.0) === "30.77%". */
const p = (s: number): string => `${((s / LOOP_S) * 100).toFixed(2)}%`;

/**
 * Rows condensed from the real medical protocol shipped in
 * GoldenHour.tsx DEFAULT_PROTOCOLS. Checks land paced proportionally to
 * their timeframes (4.0s / 4.9s / 6.0s / 7.4s per spec).
 */
const STEPS = [
  { name: 'Call 911', window: '0–1 min', critical: true, slide: 1.6, check: 4.0 },
  { name: 'CPR / stop bleeding', window: '1–3 min', critical: true, slide: 2.05, check: 4.9 },
  { name: 'Share ICE card with responders', window: '3–5 min', critical: false, slide: 2.5, check: 6.0 },
  { name: 'Contact Primary trustee', window: '5–10 min', critical: false, slide: 2.95, check: 7.4 },
] as const;

/** CRITICAL badge pop times: 150ms after rows 1–2 slide in. */
const BADGE_POP = [1.75, 2.2] as const;

/** Counter cross-fade windows: `n/4` appears as its check lands. */
const COUNTS: ReadonlyArray<{ label: string; fadeIn: number; fadeOut: number | null }> = [
  { label: '0 / 4', fadeIn: 3.3, fadeOut: 4.0 },
  { label: '1 / 4', fadeIn: 4.0, fadeOut: 4.9 },
  { label: '2 / 4', fadeIn: 4.9, fadeOut: 6.0 },
  { label: '3 / 4', fadeIn: 6.0, fadeOut: 7.4 },
  { label: '4 / 4', fadeIn: 7.4, fadeOut: null },
];

/* ------------------------------------------------------------------ */
/* Scoped CSS (lsgh- prefix; no collisions with the shared registry)   */
/* ------------------------------------------------------------------ */

const rowKeyframes = (i: number, slide: number, check: number): string => `
@keyframes lsgh-row${i} {
  0%, ${p(slide)} { opacity: 0; transform: translateX(-16px); }
  ${p(slide + 0.4)}, 100% { opacity: 1; transform: translateX(0); }
}
@keyframes lsgh-txt${i} {
  0%, ${p(check)} { opacity: 0.5; }
  ${p(check + 0.45)}, 100% { opacity: 0.95; }
}
@keyframes lsgh-tag${i} {
  0%, ${p(check)} { opacity: 0.55; }
  ${p(check + 0.3)} { opacity: 1; }
  ${p(check + 0.9)}, 100% { opacity: 0.55; }
}
@keyframes lsgh-fill${i} {
  0%, ${p(check)} { opacity: 0; }
  ${p(check + 0.25)}, 100% { opacity: 1; }
}
@keyframes lsgh-chk${i} {
  0%, ${p(check)} { stroke-dashoffset: 20; }
  ${p(check + 0.25)}, 100% { stroke-dashoffset: 0; }
}`;

const CSS = `
.lsgh-fig { margin: 0; }
.lsgh-fig text { font-family: 'Sora', system-ui, sans-serif; }
.lsgh-motion-only { opacity: 0; }
.lsgh-badge { transform-box: fill-box; transform-origin: center; }
.lsgh-barfill { transform-box: fill-box; transform-origin: left center; }

/* -- run-once intro elements (persist across the 13s loop) -- */
@keyframes lsgh-spine-draw { from { stroke-dashoffset: 288; } to { stroke-dashoffset: 0; } }
@keyframes lsgh-fadeup {
  from { opacity: 0; transform: translateY(8px); }
  to { opacity: 1; transform: translateY(0); }
}
@keyframes lsgh-pulse-trace {
  0% { stroke-dashoffset: 320; opacity: 0.9; }
  60% { stroke-dashoffset: 0; opacity: 0.9; }
  100% { stroke-dashoffset: 0; opacity: 0.25; }
}

/* -- 13s loop keyframes (percent-of-loop; never animation-delay) -- */
@keyframes lsgh-barin { 0%, ${p(3.3)} { opacity: 0; } ${p(3.7)}, 100% { opacity: 1; } }
@keyframes lsgh-bar {
  0%, ${p(4.0)} { transform: scaleX(0); }
  ${p(4.3)}, ${p(4.9)} { transform: scaleX(0.25); }
  ${p(5.2)}, ${p(6.0)} { transform: scaleX(0.5); }
  ${p(6.3)}, ${p(7.4)} { transform: scaleX(0.75); }
  ${p(7.7)}, 100% { transform: scaleX(1); }
}
@keyframes lsgh-glow { 0%, ${p(8.0)} { opacity: 0; } ${p(8.3)} { opacity: 0.5; } ${p(8.6)}, 100% { opacity: 0; } }
@keyframes lsgh-endfade { 0%, ${p(12.2)} { opacity: 1; } 100% { opacity: 0; } }
${STEPS.map((s, i) => rowKeyframes(i, s.slide, s.check)).join('')}
${BADGE_POP.map(
  (t, i) => `
@keyframes lsgh-badge${i} {
  0%, ${p(t)} { opacity: 0; transform: scale(0.6); }
  ${p(t + 0.3)}, 100% { opacity: 1; transform: scale(1); }
}`,
).join('')}
${COUNTS.map(
  (c, i) => `
@keyframes lsgh-n${i} {
  0%, ${p(c.fadeIn)} { opacity: 0; }
  ${
    c.fadeOut == null
      ? `${p(c.fadeIn + 0.3)}, 100% { opacity: 1; }`
      : `${p(c.fadeIn + 0.3)}, ${p(c.fadeOut)} { opacity: 1; }
  ${p(c.fadeOut + 0.3)}, 100% { opacity: 0; }`
  }
}`,
).join('')}

/* -- bindings: animations run ONLY inside .lsgh-anim.lsgh-play -- */
.lsgh-anim.lsgh-play .lsgh-loop {
  animation-duration: ${LOOP_S}s;
  animation-timing-function: ease-out;
  animation-iteration-count: infinite;
  animation-fill-mode: both;
}
.lsgh-anim.lsgh-play .lsgh-spine { animation: lsgh-spine-draw 1s ease-out both; }
.lsgh-anim.lsgh-play .lsgh-tickg { animation: lsgh-fadeup 0.4s ease-out both; }
.lsgh-anim.lsgh-play .lsgh-pulse { animation: lsgh-pulse-trace 1.4s ease-out 0.9s both; }
.lsgh-anim.lsgh-play .lsgh-cap { animation: lsgh-fadeup 0.5s ease-out 8.2s both; }
${STEPS.map(
  (_, i) => `
.lsgh-anim.lsgh-play .lsgh-r${i} { animation-name: lsgh-row${i}; }
.lsgh-anim.lsgh-play .lsgh-t${i} { animation-name: lsgh-txt${i}; }
.lsgh-anim.lsgh-play .lsgh-g${i} { animation-name: lsgh-tag${i}; }
.lsgh-anim.lsgh-play .lsgh-f${i} { animation-name: lsgh-fill${i}; }
.lsgh-anim.lsgh-play .lsgh-k${i} { animation-name: lsgh-chk${i}; }`,
).join('')}
${BADGE_POP.map(
  (_, i) => `
.lsgh-anim.lsgh-play .lsgh-b${i} { animation-name: lsgh-badge${i}; animation-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1); }`,
).join('')}
${COUNTS.map(
  (_, i) => `
.lsgh-anim.lsgh-play .lsgh-cn${i} { animation-name: lsgh-n${i}; }`,
).join('')}
.lsgh-anim.lsgh-play .lsgh-barwrap { animation-name: lsgh-barin; }
.lsgh-anim.lsgh-play .lsgh-barfill { animation-name: lsgh-bar; }
.lsgh-anim.lsgh-play .lsgh-glowr { animation-name: lsgh-glow; }
.lsgh-anim.lsgh-play .lsgh-end { animation-name: lsgh-endfade; }

/* -- reduced motion: kill everything; rest state IS the final frame -- */
@media (prefers-reduced-motion: reduce) {
  .lsgh-anim, .lsgh-anim * { animation: none !important; transition: none !important; }
}
`;

/* ------------------------------------------------------------------ */
/* In-view play gating                                                 */
/* ------------------------------------------------------------------ */

function useInViewOnce(): { ref: React.RefObject<HTMLElement | null>; playing: boolean } {
  const ref = useRef<HTMLElement | null>(null);
  const [playing, setPlaying] = useState(false);

  useEffect(() => {
    const el = ref.current;
    if (!el || typeof IntersectionObserver === 'undefined') {
      setPlaying(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setPlaying(true);
            observer.disconnect();
          }
        }
      },
      { threshold: 0.35 },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return { ref, playing };
}

/* ------------------------------------------------------------------ */
/* Geometry                                                            */
/* ------------------------------------------------------------------ */

const SPINE_X = 56;
const ROW_X = 88;
const ROW_Y = [100, 164, 228, 292] as const;
const TICKS = [
  { y: 64, label: '0 min' },
  { y: 136, label: '15' },
  { y: 208, label: '30' },
  { y: 280, label: '45' },
  { y: 352, label: '60 min' },
] as const;

const BASE_STROKE = 'rgba(255,255,255,0.14)';
const LABEL_FILL = '#9ca3af';

const CAPTION =
  'A sample medical protocol — eight timed steps per scenario; three scenarios ship with the demo.';

/* ------------------------------------------------------------------ */
/* Component                                                           */
/* ------------------------------------------------------------------ */

export default function GoldenHourExplainer() {
  const { ref, playing } = useInViewOnce();

  return (
    <>
      <style>{CSS}</style>
      <figure
        ref={ref}
        className={`lsgh-fig lsgh-anim${playing ? ' lsgh-play' : ''}`}
        role="img"
        aria-label="A sample medical golden-hour protocol: four timed steps check off across the first sixty minutes — call 911, CPR or stop bleeding, share your ICE card with responders, contact your Primary trustee — with the critical steps flagged."
      >
        <svg viewBox="0 0 520 440" className="w-full h-auto" aria-hidden="true">
          <defs>
            <linearGradient id="lsghBarGrad" x1="0" y1="0" x2="1" y2="0">
              <stop offset="0" stopColor="#3b82f6" />
              <stop offset="1" stopColor="#22d3ee" />
            </linearGradient>
          </defs>

          {/* F — heart-pulse line behind the header row (traces once, settles at 25%) */}
          <polyline
            className="lsgh-pulse"
            points="250,30 330,30 342,30 350,16 358,44 366,22 372,30 496,30"
            fill="none"
            stroke="#ef4444"
            strokeWidth="1.5"
            strokeLinejoin="round"
            strokeLinecap="round"
            strokeDasharray="320"
            strokeDashoffset="0"
            opacity="0.25"
          />
          <text x="24" y="34" fontSize="11" letterSpacing="2" fill={LABEL_FILL} opacity="0.8">
            MEDICAL · SAMPLE PROTOCOL
          </text>

          {/* A — timeline spine, 0 min → 60 min */}
          <line
            className="lsgh-spine"
            x1={SPINE_X}
            y1={64}
            x2={SPINE_X}
            y2={352}
            stroke={BASE_STROKE}
            strokeWidth="2"
            strokeDasharray="288"
            strokeDashoffset="0"
          />
          {TICKS.map((tick, i) => (
            <g
              key={tick.label}
              className="lsgh-tickg"
              style={{ animationDelay: `${0.2 + i * 0.2}s` }}
            >
              <line
                x1={SPINE_X - 6}
                y1={tick.y}
                x2={SPINE_X + 6}
                y2={tick.y}
                stroke={BASE_STROKE}
                strokeWidth="2"
              />
              <text x={SPINE_X - 12} y={tick.y + 4} fontSize="11" fill={LABEL_FILL} textAnchor="end">
                {tick.label}
              </text>
            </g>
          ))}

          {/* G — 60:00 cap label at spine foot (fades in at the completion beat) */}
          <text
            className="lsgh-cap"
            x={SPINE_X}
            y={378}
            fontSize="11"
            fill={LABEL_FILL}
            textAnchor="middle"
          >
            60:00
          </text>

          {/* Everything below fades at 12.2–13.0s; the spine persists across loops */}
          <g className="lsgh-loop lsgh-end">
            {/* B/C/D — protocol rows, checks, CRITICAL badges */}
            {STEPS.map((step, i) => {
              const y = ROW_Y[i];
              return (
                <g key={step.name} className={`lsgh-loop lsgh-r${i}`}>
                  {/* check circle ring */}
                  <circle
                    cx={ROW_X + 16}
                    cy={y}
                    r="11"
                    fill="none"
                    stroke="rgba(255,255,255,0.28)"
                    strokeWidth="1.5"
                  />
                  {/* circle fill lands with the check */}
                  <circle
                    className={`lsgh-loop lsgh-f${i}`}
                    cx={ROW_X + 16}
                    cy={y}
                    r="10"
                    fill="rgba(59,130,246,0.22)"
                  />
                  {/* checkmark stroke draw */}
                  <path
                    className={`lsgh-loop lsgh-k${i}`}
                    d={`M${ROW_X + 11.5} ${y + 0.5} L${ROW_X + 15} ${y + 4} L${ROW_X + 22} ${y - 4.5}`}
                    fill="none"
                    stroke="#60a5fa"
                    strokeWidth="2.2"
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeDasharray="20"
                    strokeDashoffset="0"
                  />
                  {/* step name — dim until its check lands */}
                  <text
                    className={`lsgh-loop lsgh-t${i}`}
                    x={ROW_X + 40}
                    y={y - 1}
                    fontSize="13"
                    fill="#ffffff"
                    opacity="0.95"
                  >
                    {step.name}
                  </text>
                  {/* timeframe window tag */}
                  <text
                    className={`lsgh-loop lsgh-g${i}`}
                    x={ROW_X + 40}
                    y={y + 16}
                    fontSize="10.5"
                    fill={LABEL_FILL}
                    opacity="0.55"
                  >
                    {step.window}
                  </text>
                  {/* D — CRITICAL badge (rows 1–2 only; the page's only amber here) */}
                  {step.critical && (
                    <g className={`lsgh-loop lsgh-badge lsgh-b${i}`}>
                      <rect
                        x={428}
                        y={y - 10}
                        width="66"
                        height="19"
                        rx="9.5"
                        fill="rgba(245,158,11,0.08)"
                        stroke="#f59e0b"
                        strokeOpacity="0.7"
                        strokeWidth="1"
                      />
                      <text
                        x={461}
                        y={y + 3.5}
                        fontSize="9"
                        letterSpacing="1"
                        fill="#f59e0b"
                        textAnchor="middle"
                      >
                        CRITICAL
                      </text>
                    </g>
                  )}
                </g>
              );
            })}

            {/* E — progress bar + stacked-numeral counter */}
            <g className="lsgh-loop lsgh-barwrap">
              {COUNTS.map((count, i) => (
                <text
                  key={count.label}
                  className={`lsgh-loop lsgh-cn${i}${count.fadeOut == null ? '' : ' lsgh-motion-only'}`}
                  x={496}
                  y={388}
                  fontSize="12"
                  fill={LABEL_FILL}
                  textAnchor="end"
                >
                  {count.label}
                </text>
              ))}
              <rect x={ROW_X} y={396} width="408" height="8" rx="4" fill="rgba(255,255,255,0.08)" />
              <rect
                className="lsgh-loop lsgh-barfill"
                x={ROW_X}
                y={396}
                width="408"
                height="8"
                rx="4"
                fill="url(#lsghBarGrad)"
              />
              {/* completion glow pulse — motion-only */}
              <rect
                className="lsgh-loop lsgh-glowr lsgh-motion-only"
                x={ROW_X - 2}
                y={394}
                width="412"
                height="12"
                rx="6"
                fill="none"
                stroke="#22d3ee"
                strokeWidth="2"
              />
            </g>
          </g>
        </svg>
        <figcaption className="text-xs text-white/60 mt-4">{CAPTION}</figcaption>
      </figure>
    </>
  );
}
