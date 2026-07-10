// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

import { useEffect, useState } from 'react';
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
import {
  normalizePublicTrustRouteContract,
  type PublicTrustRouteContract,
} from '../components/landing/publicTrustDisplayCopy';

const TRUST_STATUS_ROUTE = '/api/trust/status';

/**
 * Landing page — thin assembler only. Section order per build spec v2.0:
 * Header → Hero → Trust strip → Explainers (E1 ICE / E2 PACE / E3 Golden hour)
 * → Features → Under the hood → What we don't do → Final CTA → Footer.
 */
export default function Landing() {
  const [publicTrustStatus, setPublicTrustStatus] =
    useState<PublicTrustRouteContract | null>(null);

  useEffect(() => {
    let isMounted = true;
    const controller = new AbortController();

    async function loadPublicTrustStatus() {
      try {
        const response = await fetch(TRUST_STATUS_ROUTE, {
          headers: { Accept: 'application/json' },
          signal: controller.signal,
        });

        if (!isMounted) {
          return;
        }

        if (!response.ok) {
          setPublicTrustStatus(null);
          return;
        }

        const payload: unknown = await response.json();

        if (isMounted) {
          setPublicTrustStatus(normalizePublicTrustRouteContract(payload));
        }
      } catch {
        if (isMounted) {
          setPublicTrustStatus(null);
        }
      }
    }

    void loadPublicTrustStatus();

    return () => {
      isMounted = false;
      controller.abort();
    };
  }, []);

  return (
    <div className="min-h-screen bg-gradient-to-b from-[#0a0a10] via-[#0a1628] to-[#0a0a10]">
      <Header />
      <main>
        <Hero />
        <TrustStrip trustStatus={publicTrustStatus} />
        <Explainers />
        <FeaturesGrid />
        <UnderTheHood trustStatus={publicTrustStatus} />
        <HonestyBlock />
        <FinalCta />
      </main>
      <Footer />
    </div>
  );
}
