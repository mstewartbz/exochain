export const EXOCHAIN_SETTLEMENT_AUTHORITY = 'EXOCHAIN';

const CORE_ROUTES = {
  missions: '/api/v1/economy/missions',
  contributionReceipts: '/api/v1/economy/contribution-receipts',
  legacyReceipts: '/api/v1/economy/legacy-receipts',
  rulesets: '/api/v1/economy/rulesets',
  contributionNodes: '/api/v1/economy/contribution-nodes',
  contributionOffers: '/api/v1/economy/contribution-offers',
};

function normalizeBaseUrl(value) {
  return String(value || '').trim().replace(/\/+$/, '');
}

function requireNonEmpty(value, field) {
  const trimmed = String(value || '').trim();
  if (!trimmed) {
    throw new Error(`${field} is required`);
  }
  return trimmed;
}

function proposedBasisLines(lines) {
  if (!Array.isArray(lines) || lines.length === 0) {
    return [];
  }
  return lines.map((line, index) => ({
    basis: requireNonEmpty(line.basis, `proposedBasis[${index}].basis`),
    share_bp: Number.parseInt(String(line.share_bp), 10),
  }));
}

export function generateLegacyReceiptProposal(input) {
  const upstreamProject = requireNonEmpty(input.upstreamProject, 'upstreamProject');
  const receivingSystem = requireNonEmpty(input.receivingSystem, 'receivingSystem');
  const license = requireNonEmpty(input.license, 'license');
  const sourceUri = requireNonEmpty(input.sourceUri, 'sourceUri');
  const materialityTier = requireNonEmpty(input.materialityTier, 'materialityTier');

  return {
    settlement_authority: EXOCHAIN_SETTLEMENT_AUTHORITY,
    local_settlement_authority: false,
    generated_by: 'ExoForge HonorGood factory adapter',
    required_review: [
      'human materiality review',
      'contributor acceptance if settlement is requested',
      'human ratification before any ratified agreement status',
    ],
    legacy_receipt: {
      contributor: {
        ProjectTreasury: {
          project: upstreamProject,
          treasury_ref: `public-project-treasury:${upstreamProject}`,
        },
      },
      contribution_name: upstreamProject,
      contribution_type: 'upstream contribution',
      source_uri: sourceUri,
      license,
      receiving_system: receivingSystem,
      materiality_tier: materialityTier,
      attribution_required: true,
      settlement_eligible: false,
      economic_ruleset_id: null,
      beneficiary: {
        beneficiary_type: 'ProjectTreasury',
        reference: {
          ProjectTreasury: {
            project: upstreamProject,
            treasury_ref: `public-project-treasury:${upstreamProject}`,
          },
        },
      },
      active_while_materially_used: true,
      legal_effect: 'VoluntaryRecognitionOnly',
      status: 'Proposed',
      signed_contributor_acceptance_hash: null,
      human_ratifier_did: null,
    },
    proposed_terms: {
      treatment: 'evergreen attribution; conditional participation only if accepted and ratified in EXOCHAIN core',
      no_current_legal_obligation_claimed: true,
      basis_lines: proposedBasisLines(input.proposedBasis),
    },
  };
}

export class ExoForgeHonorGoodClient {
  constructor(options = {}) {
    this.baseUrl = normalizeBaseUrl(options.baseUrl ?? process.env.EXOCHAIN_API_BASE_URL);
    this.apiToken = options.apiToken ?? process.env.EXOCHAIN_API_TOKEN ?? '';
    this.fetchImpl = options.fetchImpl ?? globalThis.fetch;
  }

  configured() {
    return Boolean(this.baseUrl);
  }

  status() {
    return {
      configured: this.configured(),
      settlement_authority: EXOCHAIN_SETTLEMENT_AUTHORITY,
      local_settlement_authority: false,
    };
  }

  async submitMission(payload) {
    return this.#post(CORE_ROUTES.missions, payload);
  }

  async submitContributionReceipt(payload) {
    return this.#post(CORE_ROUTES.contributionReceipts, payload);
  }

  async submitLegacyReceipt(payload) {
    return this.#post(CORE_ROUTES.legacyReceipts, payload);
  }

  async submitRuleset(payload) {
    return this.#post(CORE_ROUTES.rulesets, payload);
  }

  async submitContributionNode(payload) {
    return this.#post(CORE_ROUTES.contributionNodes, payload);
  }

  async submitContributionOffer(payload) {
    return this.#post(CORE_ROUTES.contributionOffers, payload);
  }

  async #post(path, payload) {
    if (!this.baseUrl) {
      throw new Error('EXOCHAIN_API_BASE_URL is required; ExoForge cannot record HonorGood objects locally');
    }
    if (typeof this.fetchImpl !== 'function') {
      throw new Error('fetch is required to reach EXOCHAIN core');
    }
    const headers = {
      'Content-Type': 'application/json',
      Accept: 'application/json',
    };
    if (this.apiToken) {
      headers.Authorization = `Bearer ${this.apiToken}`;
    }
    const response = await this.fetchImpl(`${this.baseUrl}${path}`, {
      method: 'POST',
      headers,
      body: JSON.stringify(payload),
    });
    if (!response.ok) {
      throw new Error(`EXOCHAIN economy API rejected request with status ${response.status}`);
    }
    return response.json();
  }
}
