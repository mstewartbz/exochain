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

import './landing.css';

/**
 * Static hero-object SVG: the digital ICE card. No animation classes other
 * than an idle `ls-float` on the wrapper (static under reduced motion).
 * All card data is abstract placeholder bars — never fake personal data.
 */
export default function IceCardArt() {
  return (
    <div className="relative ls-anim play" aria-hidden={false}>
      {/* Soft glow ellipse behind the card */}
      <div
        className="absolute inset-4 bg-blue-500/20 blur-3xl rounded-full"
        aria-hidden="true"
      />
      <svg
        viewBox="0 0 360 224"
        className="relative w-full h-auto max-w-md mx-auto drop-shadow-2xl"
        style={{ animation: 'ls-float 6s ease-in-out infinite' }}
        xmlns="http://www.w3.org/2000/svg"
        role="img"
        aria-label="A digital ICE card: gold star-of-life emblem, red EMS cross, placeholder identity bars, and a QR code block."
      >
        {/* Card body */}
        <rect x="2" y="2" width="356" height="220" rx="20" fill="#0a1a5e" stroke="rgba(255,255,255,0.14)" strokeWidth="1.5" />
        <rect x="2" y="2" width="356" height="220" rx="20" fill="url(#ls-card-sheen)" />
        <defs>
          <linearGradient id="ls-card-sheen" x1="0" y1="0" x2="1" y2="1">
            <stop offset="0%" stopColor="rgba(255,255,255,0.06)" />
            <stop offset="60%" stopColor="rgba(255,255,255,0)" />
          </linearGradient>
        </defs>

        {/* Gold star (Maltese-cross star), top-left */}
        <g transform="translate(38,40)" fill="#eab308">
          <rect x="-5" y="-18" width="10" height="36" rx="3" />
          <rect x="-18" y="-5" width="36" height="10" rx="3" />
          <g transform="rotate(45)">
            <rect x="-4" y="-15" width="8" height="30" rx="3" opacity="0.85" />
            <rect x="-15" y="-4" width="30" height="8" rx="3" opacity="0.85" />
          </g>
          <circle r="6.5" fill="#0a1a5e" />
          <circle r="6.5" fill="none" stroke="#eab308" strokeWidth="2" />
        </g>

        {/* Card title */}
        <text x="70" y="34" fontFamily="Sora, sans-serif" fontWeight="700" fontSize="16" fill="#ffffff">
          ICE
        </text>
        <text x="70" y="50" fontFamily="Sora, sans-serif" fontSize="8" letterSpacing="1.5" fill="#9ca3af">
          IN CASE OF EMERGENCY
        </text>

        {/* Red EMS cross, top-right */}
        <g transform="translate(322,36)" fill="#ef4444">
          <rect x="-5" y="-14" width="10" height="28" rx="2.5" />
          <rect x="-14" y="-5" width="28" height="10" rx="2.5" />
        </g>

        {/* Chip */}
        <g transform="translate(28,78)">
          <rect width="40" height="30" rx="6" fill="rgba(255,255,255,0.16)" stroke="rgba(255,255,255,0.25)" />
          <path d="M0 15 H40 M13 0 V30 M27 0 V30" stroke="rgba(10,26,94,0.9)" strokeWidth="1.5" fill="none" />
        </g>

        {/* Name / blood-type placeholder bars (abstract, no real data) */}
        <rect x="28" y="124" width="128" height="10" rx="5" fill="rgba(255,255,255,0.28)" />
        <rect x="28" y="144" width="88" height="8" rx="4" fill="rgba(255,255,255,0.16)" />
        <rect x="28" y="162" width="56" height="8" rx="4" fill="rgba(255,255,255,0.16)" />
        <text x="28" y="196" fontFamily="Sora, sans-serif" fontSize="8" letterSpacing="1.5" fill="rgba(255,255,255,0.35)">
          BLOOD TYPE · ALLERGIES · CONTACTS
        </text>

        {/* QR block: white panel with navy modules (~24 squares) */}
        <g transform="translate(244,96)">
          <rect width="88" height="88" rx="8" fill="#ffffff" />
          {/* Finder patterns */}
          <rect x="8" y="8" width="18" height="18" fill="#0a1a5e" />
          <rect x="12" y="12" width="10" height="10" fill="#ffffff" />
          <rect x="15" y="15" width="4" height="4" fill="#0a1a5e" />
          <rect x="62" y="8" width="18" height="18" fill="#0a1a5e" />
          <rect x="66" y="12" width="10" height="10" fill="#ffffff" />
          <rect x="69" y="15" width="4" height="4" fill="#0a1a5e" />
          <rect x="8" y="62" width="18" height="18" fill="#0a1a5e" />
          <rect x="12" y="66" width="10" height="10" fill="#ffffff" />
          <rect x="15" y="69" width="4" height="4" fill="#0a1a5e" />
          {/* Data modules */}
          <g fill="#0a1a5e">
            <rect x="34" y="8" width="6" height="6" />
            <rect x="46" y="8" width="6" height="6" />
            <rect x="34" y="20" width="6" height="6" />
            <rect x="40" y="26" width="6" height="6" />
            <rect x="52" y="20" width="6" height="6" />
            <rect x="8" y="34" width="6" height="6" />
            <rect x="20" y="34" width="6" height="6" />
            <rect x="32" y="34" width="6" height="6" />
            <rect x="44" y="38" width="6" height="6" />
            <rect x="56" y="34" width="6" height="6" />
            <rect x="68" y="34" width="6" height="6" />
            <rect x="14" y="46" width="6" height="6" />
            <rect x="26" y="46" width="6" height="6" />
            <rect x="38" y="50" width="6" height="6" />
            <rect x="50" y="46" width="6" height="6" />
            <rect x="62" y="46" width="6" height="6" />
            <rect x="74" y="46" width="6" height="6" />
            <rect x="34" y="62" width="6" height="6" />
            <rect x="46" y="58" width="6" height="6" />
            <rect x="58" y="62" width="6" height="6" />
            <rect x="70" y="58" width="6" height="6" />
            <rect x="34" y="74" width="6" height="6" />
            <rect x="46" y="70" width="6" height="6" />
            <rect x="58" y="74" width="6" height="6" />
            <rect x="70" y="70" width="6" height="6" />
          </g>
        </g>
      </svg>
    </div>
  );
}
