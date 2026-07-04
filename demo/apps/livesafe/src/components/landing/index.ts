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
