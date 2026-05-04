import { DocPage } from '@/components/content/DocPage';

export const metadata = { title: 'Glossary' };

const TERMS: { term: string; def: string }[] = [
  { term: 'AVC', def: 'Autonomous Volition Credential. Signed credential declaring what an actor may pursue.' },
  { term: 'Actor', def: 'Human, organization, agent, holon, service, or validator registered on EXOCHAIN.' },
  { term: 'Holon', def: 'A composite actor — typically a multi-organization participant — that acts as both whole and part within EXOCHAIN.' },
  { term: 'Custody-native blockchain', def: 'A blockchain whose primary purpose is preserving chain-of-custody, not coin issuance.' },
  { term: 'Custody verifier', def: 'A validator role: produces blocks while attesting custody.' },
  { term: 'Trust receipt', def: 'Hash-chained, signed record of an authorized (or denied) action.' },
  { term: 'Settlement receipt', def: 'Receipt for the economic layer. Currently amount = 0 with ZeroFeeReason.' },
  { term: 'ZeroFeeReason', def: 'Explicit reason for a zero-priced settlement under the launch policy.' },
  { term: 'Policy domain', def: 'Named set of actions and constraints under which an AVC is valid.' },
  { term: 'Revocation cascade', def: 'Effect of revoking a parent AVC: all derivative credentials inherit revocation.' }
];

export default function Page() {
  return (
    <DocPage title="Glossary">
      <dl className="space-y-4">
        {TERMS.map((t) => (
          <div key={t.term}>
            <dt className="font-semibold">{t.term}</dt>
            <dd className="text-ink/80 dark:text-vellum-soft/80">{t.def}</dd>
          </div>
        ))}
      </dl>
    </DocPage>
  );
}
