import ExplainerSection from './ExplainerSection';
import IceCardExplainer from '../explainers/IceCardExplainer';
import PaceNetworkExplainer from '../explainers/PaceNetworkExplainer';
import GoldenHourExplainer from '../explainers/GoldenHourExplainer';

/**
 * The three landing explainers, in narrative order:
 * E1 ICE card (copy left) → E2 PACE (figure left) → E3 Golden hour (copy left).
 * Copy hedge contract: protocol-defined-but-unwired features stay hedged
 * ("designed to / the architecture defines / (spec)") — do not strip.
 */
export default function Explainers() {
  return (
    <>
      {/* E1 — ICE card */}
      <ExplainerSection
        id="ice"
        eyebrow="THE ICE CARD"
        title="When a stranger needs to save your life."
        body={
          <p>
            Your ICE card holds what a first responder needs — blood type,
            allergies, medications, who to call. The scan flow is designed
            around consent: a responder identifies themselves to unlock the
            card, and their access carries an expiry time. You decide
            what&rsquo;s on the card. The protocol is designed so the card
            decides who sees it, and for how long.
          </p>
        }
        triad={['Blood type & allergies', 'Emergency contacts', 'Time-limited responder access']}
        spec={{
          code: 'POST /ice-card/scan/:token { responder_did }\n  → { card, consent_expires_at_ms }',
          caption: 'The consent contract, verbatim. Identified requester in, expiring grant out.',
        }}
        microCta={{ label: 'Build your ICE card →', to: '/register' }}
        figure={<IceCardExplainer />}
      />

      {/* E2 — PACE network (figure left) */}
      <ExplainerSection
        flip
        eyebrow="THE PACE NETWORK"
        title="Four people you'd call at 3 a.m."
        body={
          <p>
            PACE is a military communications discipline — Primary, Alternate,
            Contingency, Emergency — turned into a trust structure. You name
            four people, in order. The architecture defines a threshold model:
            your recovery key would split into four shards, with no fewer than
            three trustees together able to act on your behalf. No single
            person — not even your Primary, not even LiveSafe — is designed to
            hold enough to act alone. The design distributes trust by math,
            not by promise.
          </p>
        }
        triad={['Four named roles', '3-of-4 threshold by design', 'Designed so every trustee names their own four']}
        spec={{
          code: 'Key sharding: Shamir threshold scheme (spec)\nThreshold: 3 of 4 · Unilateral access: 0 of 4',
          caption: 'Threshold custody as defined in the LiveSafe protocol.',
        }}
        microCta={{ label: 'Name your four →', to: '/register' }}
        figure={<PaceNetworkExplainer />}
      />

      {/* E3 — Golden hour */}
      <ExplainerSection
        eyebrow="THE GOLDEN HOUR"
        title="The first sixty minutes decide the most."
        body={
          <p>
            Emergency medicine calls the first hour after trauma the golden
            hour. LiveSafe adapts the idea for civil defense: step-by-step
            checklists for natural disasters, medical emergencies, and civil
            unrest — each step tagged with a target timeframe and the critical
            ones flagged. When adrenaline erases your ability to think, a
            checklist thinks for you.
          </p>
        }
        triad={['Scenario checklists', 'Timeframe-tagged steps', 'Critical steps flagged']}
        spec={{
          code: 'medical.protocol: 8 steps · timeframe-tagged\nstep[0] = { "Call 911", window: "0-1 min", critical: true }',
          caption: 'Protocols ship as structured data, not advice.',
        }}
        microCta={{ label: 'See the protocols →', to: '/register' }}
        figure={<GoldenHourExplainer />}
      />
    </>
  );
}
