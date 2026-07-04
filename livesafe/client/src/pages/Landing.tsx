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
  Header,
  Hero,
  TrustStrip,
  Explainers,
  FeaturesGrid,
  UnderTheHood,
  HonestyBlock,
  FinalCta,
  Footer,
} from '../components/landing';

/**
 * Landing page — thin assembler only. Section order per build spec v2.0:
 * Header → Hero → Trust strip → Explainers (E1 ICE / E2 PACE / E3 Golden hour)
 * → Features → Under the hood → What we don't do → Final CTA → Footer.
 */
export default function Landing() {
  return (
    <div className="min-h-screen bg-gradient-to-b from-[#0a0a10] via-[#0a1628] to-[#0a0a10]">
      <Header />
      <main>
        <Hero />
        <TrustStrip />
        <Explainers />
        <FeaturesGrid />
        <UnderTheHood />
        <HonestyBlock />
        <FinalCta />
      </main>
      <Footer />
    </div>
  );
}
