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
  getLandingPublicTrustDisplayCopy,
  type PublicTrustRouteContract,
} from './publicTrustDisplayCopy';

/** Small hex-node glyph for the EXOCHAIN item. */
function HexNode() {
  return (
    <svg
      viewBox="0 0 16 16"
      width="14"
      height="14"
      className="inline-block text-blue-400"
      aria-hidden="true"
    >
      <path
        d="M8 1.5 13.6 4.75v6.5L8 14.5 2.4 11.25v-6.5L8 1.5Z"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.4"
      />
      <circle cx="8" cy="8" r="1.6" fill="currentColor" />
    </svg>
  );
}

interface TrustStripProps {
  trustStatus?: PublicTrustRouteContract | null;
}

export default function TrustStrip({ trustStatus = null }: TrustStripProps) {
  const copy = getLandingPublicTrustDisplayCopy(trustStatus);

  return (
    <section className="bg-white/[0.02] border-y border-white/[0.06] py-6">
      <div className="max-w-6xl mx-auto px-6 md:px-8">
        <div className="flex flex-wrap gap-x-10 gap-y-3 items-center text-sm text-gray-500">
          <span className="flex items-center gap-2 text-gray-300">
            <HexNode />
            <span>
              <span className="font-medium">{copy.trustStripLead}</span>
              {' - '}
              {copy.trustStripDetail}
            </span>
          </span>
          {copy.trustStripItems.map((item) => (
            <span key={item}>{item}</span>
          ))}
        </div>
      </div>
    </section>
  );
}
