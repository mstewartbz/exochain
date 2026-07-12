// Copyright (c) 2026 Exochain Foundation. All rights reserved.
// Proprietary and confidential. See livesafe/LICENSE.
// SPDX-License-Identifier: LicenseRef-Exochain-Proprietary

// Landing page section barrel. Assembly order for Landing.tsx:
// Header → Hero → TrustStrip → Explainers (E1/E2/E3) → FeaturesGrid →
// UnderTheHood → HonestyBlock → FinalCta → Footer.
// Page root background: bg-gradient-to-b from-[#0a0a10] via-[#0a1628] to-[#0a0a10]

import './landing.css';

export { default as Header } from './Header';
export { default as Hero } from './Hero';
export { default as TrustStrip } from './TrustStrip';
export { default as Explainers } from './Explainers';
export { default as ExplainerSection } from './ExplainerSection';
export { default as FeaturesGrid } from './FeaturesGrid';
export { default as UnderTheHood } from './UnderTheHood';
export { default as HonestyBlock } from './HonestyBlock';
export { default as FinalCta } from './FinalCta';
export { default as Footer } from './Footer';
export { default as IceCardArt } from './IceCardArt';
export { useInView } from './useInView';
