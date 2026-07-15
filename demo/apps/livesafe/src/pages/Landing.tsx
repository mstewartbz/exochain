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
} from '@/components/landing';

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
