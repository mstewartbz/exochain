// IceCardExplainer — "The consent-gated scan" animated inline-SVG explainer.
//
// Storyboard (12s infinite loop): the ICE card appears locked → a scan beam
// sweeps the QR → the responder node pops in → a `responder_did` chip travels
// responder→card (identity is presented TO the card) → the padlock opens →
// the unlock panel unfolds with the payload chip `card + consent_expires_at_ms`
// → the expiry ring depletes (final stretch amber) → access lapses → loop.
//
// Motion doctrine: every element's resting CSS IS the final frame; elements
// that exist only for motion carry `.ls-motion-only { opacity: 0 }`. Under
// `prefers-reduced-motion: reduce` all animation is disabled and the figure
// degrades to its complete static diagram (card unlocked, connector drawn,
// panel open, ring at ~25% with amber tail). No SMIL, no JS animation —
// CSS keyframes only, gated by IntersectionObserver adding `.play`.

import { useEffect, useRef, useState } from 'react';

// QR module pattern — abstract placeholder data, never a real payload.
// Two alternating groups so the scan beam can "flicker" them independently.
const QR_ORIGIN_X = 142;
const QR_ORIGIN_Y = 152;
const QR_CELL = 7;
const QR_SIZE = 6;

const QR_GROUP_A: ReadonlyArray<readonly [number, number]> = [
  [0, 0], [1, 0], [2, 0], [0, 1], [2, 1], [0, 2], [1, 2], [2, 2],
  [5, 0], [7, 0], [4, 1], [6, 1], [3, 3], [5, 3],
];

const QR_GROUP_B: ReadonlyArray<readonly [number, number]> = [
  [7, 2], [4, 3], [0, 4], [2, 4], [6, 4], [1, 5], [3, 5], [5, 5],
  [7, 5], [0, 6], [4, 6], [0, 7], [2, 7], [5, 7], [7, 7],
];

const CAPTION =
  'A responder presents a DID → the card unlocks → access expires. By design.';

const STYLES = `
.ice-explainer text { user-select: none; }
.ice-explainer .ls-motion-only { opacity: 0; }
.ice-explainer .ice-font { font-family: 'Sora', 'Inter', sans-serif; }
.ice-explainer .ice-mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }

/* Animations run only once the figure has entered the viewport (.play). */
.ls-anim.play .ice-card-outer { animation: ice-card-in 0.8s ease-out both; }
.ls-anim.play .ice-card-loop { animation: ice-card-loop 12s ease-out infinite both; }
.ls-anim.play .ice-lock-closed { animation: ice-lock-closed 12s linear infinite both; }
.ls-anim.play .ice-lock-open { animation: ice-lock-open 12s linear infinite both; }
.ls-anim.play .ice-beam { animation: ice-beam 12s linear infinite both; }
.ls-anim.play .ice-qr-a { animation: ice-qr-a 12s ease-out infinite both; }
.ls-anim.play .ice-qr-b { animation: ice-qr-b 12s ease-out infinite both; }
.ls-anim.play .ice-qr-pulse {
  animation: ice-qr-pulse 12s ease-out infinite both;
  transform-box: fill-box; transform-origin: 50% 50%;
}
.ls-anim.play .ice-responder {
  animation: ice-responder 12s ease-out infinite both;
  transform-box: fill-box; transform-origin: 50% 50%;
}
.ls-anim.play .ice-responder-label { animation: ice-late-fade 12s ease-out infinite both; }
.ls-anim.play .ice-connector { animation: ice-connector 12s ease-out infinite both; }
.ls-anim.play .ice-chip { animation: ice-chip 12s ease-in-out infinite both; }
.ls-anim.play .ice-panel {
  animation: ice-panel 12s ease-out infinite both;
  transform-box: fill-box; transform-origin: 50% 0%;
}
.ls-anim.play .ice-row { animation: ice-row 12s ease-out infinite both; }
.ls-anim.play .ice-ring-group { animation: ice-ring-group 12s ease-out infinite both; }
.ls-anim.play .ice-ring-arc { animation: ice-ring-deplete 12s linear infinite both; }
.ls-anim.play .ice-ring-amber { animation: ice-ring-amber 12s linear infinite both; }
.ls-anim.play .ice-clock { animation: ice-clock 12s ease-in-out infinite both; }

/* 0.0-0.8s: card enters (one-shot). */
@keyframes ice-card-in {
  from { opacity: 0; transform: translateY(12px); }
  to { opacity: 1; transform: translateY(0); }
}

/* Card sits at 60% until the unlock beat (4.4-5.0s = 36.7-41.7%), then 100%.
   Never disappears — it is the continuity anchor across loops. */
@keyframes ice-card-loop {
  0% { opacity: 0.55; }
  6.7%, 36.7% { opacity: 0.6; }
  41.7%, 96% { opacity: 1; }
  100% { opacity: 0.55; }
}

/* Padlock: closed until the DID arrives, open after. Rest = open. */
@keyframes ice-lock-closed {
  0%, 36.7% { opacity: 1; }
  39%, 97% { opacity: 0; }
  100% { opacity: 1; }
}
@keyframes ice-lock-open {
  0%, 36.7% { opacity: 0; }
  39%, 97% { opacity: 1; }
  100% { opacity: 0; }
}

/* 1.0-2.2s: scan beam sweeps the QR twice. Motion-only. */
@keyframes ice-beam {
  0%, 8.3% { opacity: 0; transform: translateY(2px); }
  9% { opacity: 1; }
  13.2% { opacity: 1; transform: translateY(56px); }
  13.4% { opacity: 0; transform: translateY(2px); }
  14.1% { opacity: 1; }
  18% { opacity: 1; transform: translateY(56px); }
  18.3%, 100% { opacity: 0; transform: translateY(56px); }
}

/* QR module groups flicker in a 3-step stagger while the beam sweeps. */
@keyframes ice-qr-a {
  0%, 8.3% { opacity: 1; }
  10% { opacity: 0.35; }
  11.5% { opacity: 1; }
  13% { opacity: 0.35; }
  14.5% { opacity: 1; }
  16% { opacity: 0.35; }
  18.3%, 100% { opacity: 1; }
}
@keyframes ice-qr-b {
  0%, 9.1% { opacity: 1; }
  10.8% { opacity: 0.35; }
  12.3% { opacity: 1; }
  13.8% { opacity: 0.35; }
  15.3% { opacity: 1; }
  16.8% { opacity: 0.35; }
  18.3%, 100% { opacity: 1; }
}

/* 2.4-3.2s: responder node pops in; fades with everything else at loop end. */
@keyframes ice-responder {
  0% { opacity: 0; transform: scale(0.6); }
  20% { opacity: 0; transform: scale(0.6); animation-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1); }
  26.7%, 95% { opacity: 1; transform: scale(1); }
  100% { opacity: 0; transform: scale(1); }
}
@keyframes ice-late-fade {
  0%, 24% { opacity: 0; }
  29%, 95% { opacity: 1; }
  100% { opacity: 0; }
}

/* 3.2-4.4s: connector materializes as the DID chip travels responder->card.
   Fades to 30% as access lapses (10.4-11.4s), then out. Rest = drawn. */
@keyframes ice-connector {
  0%, 26.7% { opacity: 0; }
  36.7%, 86.7% { opacity: 1; }
  95% { opacity: 0.3; }
  100% { opacity: 0; }
}

/* DID chip travels responder -> card (identity presented TO the card). */
@keyframes ice-chip {
  0%, 27% { opacity: 0; transform: translate(0, 0); }
  28.5% { opacity: 1; }
  34% { opacity: 1; transform: translate(-148px, 26px); }
  35.5%, 100% { opacity: 0; transform: translate(-148px, 26px); }
}

/* 4.4-5.0s: unlock beat — one 300ms pulse on the QR outline. Motion-only. */
@keyframes ice-qr-pulse {
  0%, 36.7% { opacity: 0; transform: scale(1); }
  37.4% { opacity: 1; }
  38.3% { transform: scale(1.03); }
  39.5%, 100% { opacity: 0; transform: scale(1); }
}

/* 5.0-6.4s: unlock panel unfolds from its top edge; lapses at loop end. */
@keyframes ice-panel {
  0% { opacity: 0; transform: scale(0.85); }
  41.7% { opacity: 0; transform: scale(0.85); animation-timing-function: cubic-bezier(0.34, 1.56, 0.64, 1); }
  46.7%, 86.7% { opacity: 1; transform: scale(1); }
  95% { opacity: 0.35; }
  100% { opacity: 0; }
}

/* Payload chip + data rows fade in staggered (via animation-delay). */
@keyframes ice-row {
  0%, 47% { opacity: 0; transform: translateY(6px); }
  50.5%, 100% { opacity: 1; transform: translateY(0); }
}

/* 6.4-10.4s: expiry ring depletes linearly toward ~75% consumed.
   pathLength=100, so dashoffset 0 = full ring, 75 = 25% remaining (rest). */
@keyframes ice-ring-group {
  0%, 47% { opacity: 0; }
  53%, 95% { opacity: 1; }
  100% { opacity: 0; }
}
@keyframes ice-ring-deplete {
  0%, 53.3% { stroke-dashoffset: 0; }
  86.7%, 100% { stroke-dashoffset: 75; }
}
/* Final stretch of the remaining arc renders amber as expiry nears. */
@keyframes ice-ring-amber {
  0%, 72% { opacity: 0; }
  78%, 100% { opacity: 1; }
}
@keyframes ice-clock {
  0%, 53.3% { opacity: 1; }
  58% { opacity: 0.45; }
  62.7% { opacity: 1; }
  67.5% { opacity: 0.45; }
  72.2% { opacity: 1; }
  77% { opacity: 0.45; }
  81.7%, 100% { opacity: 1; }
}

/* Reduced motion: freeze everything at the resting (final) frame — a complete
   static diagram. Motion-only elements simply never appear. */
@media (prefers-reduced-motion: reduce) {
  .ls-anim, .ls-anim * { animation: none !important; transition: none !important; }
}
`;

export default function IceCardExplainer() {
  const figureRef = useRef<HTMLElement | null>(null);
  const [play, setPlay] = useState(false);

  useEffect(() => {
    const el = figureRef.current;
    if (!el) return;
    if (typeof IntersectionObserver === 'undefined') {
      setPlay(true);
      return;
    }
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (entry.isIntersecting) {
            setPlay(true);
            observer.disconnect();
          }
        }
      },
      { threshold: 0.35 },
    );
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  return (
    <figure
      ref={figureRef as React.Ref<HTMLElement>}
      role="img"
      aria-label={CAPTION}
      className={`ice-explainer w-full ${play ? 'ls-anim play' : 'ls-anim'}`}
    >
      <style>{STYLES}</style>
      <svg
        viewBox="0 0 520 380"
        className="w-full h-auto"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        {/* ── E · Dashed connector, card <-> responder (rest: fully drawn) ── */}
        <path
          className="ice-connector"
          d="M398 118 C 330 152, 280 152, 218 156"
          fill="none"
          stroke="#60a5fa"
          strokeWidth="1.5"
          strokeDasharray="5 5"
          strokeLinecap="round"
        />

        {/* ── C · DID chip traveling responder -> card (motion-only) ── */}
        <g className="ice-chip ls-motion-only" aria-hidden="true">
          <rect x="306" y="120" width="100" height="20" rx="10" fill="#0b1220" stroke="#22d3ee" strokeWidth="1" />
          <text className="ice-mono" x="356" y="133.5" textAnchor="middle" fontSize="10" fill="#22d3ee">
            responder_did
          </text>
        </g>

        {/* ── A · The ICE card (continuity anchor — never disappears) ── */}
        <g className="ice-card-outer">
          <g className="ice-card-loop">
            <rect x="24" y="100" width="190" height="130" rx="12" fill="#0a1a5e" stroke="rgba(255,255,255,0.14)" strokeWidth="1" />
            {/* Gold star */}
            <path
              d="M52 112 L56.5 121.5 L66 126 L56.5 130.5 L52 140 L47.5 130.5 L38 126 L47.5 121.5 Z"
              fill="#eab308"
            />
            {/* Red EMS cross */}
            <g fill="#ef4444">
              <rect x="191" y="114" width="10" height="26" rx="2" />
              <rect x="183" y="122" width="26" height="10" rx="2" />
            </g>
            {/* Chip */}
            <rect x="40" y="150" width="26" height="18" rx="3" fill="rgba(255,255,255,0.25)" />
            {/* Name / blood-type placeholder bars — abstract, never real data */}
            <rect x="40" y="182" width="84" height="7" rx="3.5" fill="rgba(255,255,255,0.35)" />
            <rect x="40" y="196" width="58" height="7" rx="3.5" fill="rgba(255,255,255,0.22)" />

            {/* White QR block, two alternating flicker groups */}
            <g className="ice-qr-a" fill="#ffffff">
              {QR_GROUP_A.map(([col, row]) => (
                <rect
                  key={`qa-${col}-${row}`}
                  x={QR_ORIGIN_X + col * QR_CELL}
                  y={QR_ORIGIN_Y + row * QR_CELL}
                  width={QR_SIZE}
                  height={QR_SIZE}
                />
              ))}
            </g>
            <g className="ice-qr-b" fill="#ffffff">
              {QR_GROUP_B.map(([col, row]) => (
                <rect
                  key={`qb-${col}-${row}`}
                  x={QR_ORIGIN_X + col * QR_CELL}
                  y={QR_ORIGIN_Y + row * QR_CELL}
                  width={QR_SIZE}
                  height={QR_SIZE}
                />
              ))}
            </g>

            {/* Unlock-beat outline pulse around the QR (motion-only) */}
            <rect
              className="ice-qr-pulse ls-motion-only"
              aria-hidden="true"
              x="138"
              y="148"
              width="66"
              height="66"
              rx="4"
              fill="none"
              stroke="#22d3ee"
              strokeWidth="1.5"
            />

            {/* Scan beam (motion-only) */}
            <line
              className="ice-beam ls-motion-only"
              aria-hidden="true"
              x1="138"
              y1="150"
              x2="204"
              y2="150"
              stroke="#22d3ee"
              strokeWidth="2"
              strokeLinecap="round"
            />

            {/* Padlock over the QR — closed (motion-only) / open (rest state) */}
            <g>
              <circle cx="171" cy="182" r="13" fill="#0a1a5e" fillOpacity="0.88" stroke="rgba(255,255,255,0.3)" strokeWidth="1" />
              {/* Closed shackle + body */}
              <g className="ice-lock-closed ls-motion-only" aria-hidden="true">
                <path d="M166.5 180 v-4 a4.5 4.5 0 0 1 9 0 v4" fill="none" stroke="#e5e7eb" strokeWidth="1.8" />
                <rect x="164.5" y="180" width="13" height="10" rx="2" fill="#e5e7eb" />
              </g>
              {/* Open shackle + body (final frame) */}
              <g className="ice-lock-open">
                <path d="M166.5 180 v-4 a4.5 4.5 0 0 1 9 -1.5" fill="none" stroke="#22d3ee" strokeWidth="1.8" />
                <rect x="164.5" y="180" width="13" height="10" rx="2" fill="#e5e7eb" />
              </g>
            </g>
          </g>
        </g>

        {/* ── D · Responder node ── */}
        <g className="ice-responder">
          <circle cx="400" cy="105" r="28" fill="rgba(255,255,255,0.03)" stroke="#60a5fa" strokeWidth="1.5" />
          {/* Helmet glyph */}
          <path d="M386 109 a14 14 0 0 1 28 0 Z" fill="rgba(96,165,250,0.25)" stroke="#60a5fa" strokeWidth="1.2" />
          <line x1="382" y1="109" x2="418" y2="109" stroke="#60a5fa" strokeWidth="1.5" strokeLinecap="round" />
          {/* Small cross on helmet */}
          <g fill="#ef4444">
            <rect x="398" y="95" width="4" height="10" rx="1" />
            <rect x="395" y="98" width="10" height="4" rx="1" />
          </g>
        </g>
        <g className="ice-responder-label ice-font">
          <text x="400" y="150" textAnchor="middle" fontSize="10" letterSpacing="1.5" fill="#9ca3af">
            RESPONDER ID
          </text>
          <text className="ice-mono" x="400" y="165" textAnchor="middle" fontSize="11" fill="#60a5fa">
            did:exo:9f2c&#8230;
          </text>
        </g>

        {/* ── G · Expiry ring + clock (rest: ~25% remaining, amber tail) ── */}
        <g className="ice-ring-group">
          <g transform="rotate(-90 336 182)">
            {/* Track */}
            <circle cx="336" cy="182" r="15" fill="none" stroke="rgba(255,255,255,0.14)" strokeWidth="3" />
            {/* Depleting arc — pathLength 100; rest offset 75 = 25% remaining */}
            <circle
              className="ice-ring-arc"
              cx="336"
              cy="182"
              r="15"
              fill="none"
              stroke="#60a5fa"
              strokeWidth="3"
              pathLength={100}
              strokeDasharray="100"
              strokeDashoffset={75}
              strokeLinecap="round"
            />
            {/* Amber final stretch of the remaining arc */}
            <circle
              className="ice-ring-amber"
              cx="336"
              cy="182"
              r="15"
              fill="none"
              stroke="#f59e0b"
              strokeWidth="3"
              pathLength={100}
              strokeDasharray="12 88"
              strokeLinecap="round"
            />
          </g>
          {/* Clock glyph */}
          <g className="ice-clock">
            <circle cx="336" cy="182" r="6.5" fill="none" stroke="#e5e7eb" strokeWidth="1.2" />
            <line x1="336" y1="182" x2="336" y2="177.5" stroke="#e5e7eb" strokeWidth="1.2" strokeLinecap="round" />
            <line x1="336" y1="182" x2="339.5" y2="184" stroke="#e5e7eb" strokeWidth="1.2" strokeLinecap="round" />
          </g>
          <text className="ice-font" x="358" y="186" textAnchor="start" fontSize="9" letterSpacing="1.5" fill="#9ca3af">
            ACCESS EXPIRES
          </text>
        </g>

        {/* ── F · Unlock panel ── */}
        <g className="ice-panel">
          <rect x="300" y="205" width="190" height="140" rx="10" fill="rgba(255,255,255,0.03)" stroke="rgba(255,255,255,0.14)" strokeWidth="1" />
          {/* Header chip: the consent payload */}
          <g className="ice-row">
            <rect x="312" y="217" width="166" height="20" rx="6" fill="rgba(34,211,238,0.08)" stroke="rgba(34,211,238,0.35)" strokeWidth="1" />
            <text className="ice-mono" x="395" y="230.5" textAnchor="middle" fontSize="9.5" fill="#22d3ee">
              card + consent_expires_at_ms
            </text>
          </g>
          {/* Data rows — abstract bars, never real values */}
          <g className="ice-row" style={{ animationDelay: '0.15s' }}>
            <text className="ice-font" x="312" y="263" fontSize="10" letterSpacing="1.5" fill="#9ca3af">
              BLOOD TYPE
            </text>
            <rect x="396" y="255" width="82" height="8" rx="4" fill="rgba(96,165,250,0.35)" />
          </g>
          <g className="ice-row" style={{ animationDelay: '0.3s' }}>
            <text className="ice-font" x="312" y="291" fontSize="10" letterSpacing="1.5" fill="#9ca3af">
              ALLERGIES
            </text>
            <rect x="396" y="283" width="82" height="8" rx="4" fill="rgba(96,165,250,0.35)" />
          </g>
          <g className="ice-row" style={{ animationDelay: '0.45s' }}>
            <text className="ice-font" x="312" y="319" fontSize="10" letterSpacing="1.5" fill="#9ca3af">
              CONTACTS
            </text>
            <rect x="396" y="311" width="82" height="8" rx="4" fill="rgba(96,165,250,0.35)" />
          </g>
        </g>
      </svg>
      <figcaption className="mt-3 text-xs text-white/60">{CAPTION}</figcaption>
    </figure>
  );
}
