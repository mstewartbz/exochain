/**
 * High-level {@link ExochainClient} — the one-stop entry point for talking to
 * an `exo-gateway` instance. Each domain API (identity, consent, governance,
 * authority) wraps the shared {@link HttpTransport}.
 */
import { HttpTransport } from './transport/http.js';
// -----------------------------------------------------------------------------
// Domain API surfaces
// -----------------------------------------------------------------------------
/** Identity-related gateway calls. */
export class IdentityApi {
    #http;
    constructor(http) {
        this.#http = http;
    }
    /** Resolve a DID to its DID document via `GET /identity/did/{did}`. */
    async resolve(did) {
        return this.#http.get(`/identity/did/${encodeURIComponent(did)}`);
    }
    /** Register a DID document via `POST /identity/did`. */
    async register(document) {
        return this.#http.post('/identity/did', document);
    }
}
/** Consent / bailment gateway calls. */
export class ConsentApi {
    #http;
    constructor(http) {
        this.#http = http;
    }
    /** Submit a bailment proposal for processing. */
    async proposeBailment(body) {
        return this.#http.post('/consent/bailment', body);
    }
    /** Fetch a bailment proposal by its content-addressed ID. */
    async getBailment(proposalId) {
        return this.#http.get(`/consent/bailment/${encodeURIComponent(proposalId)}`);
    }
}
/** Governance gateway calls. */
export class GovernanceApi {
    #http;
    constructor(http) {
        this.#http = http;
    }
    /** Create a decision via `POST /governance/decision`. */
    async createDecision(body) {
        return this.#http.post('/governance/decision', body);
    }
    /** Cast a vote on an existing decision. */
    async castVote(decisionId, body) {
        await this.#http.post(`/governance/decision/${encodeURIComponent(decisionId)}/vote`, body);
    }
    /** Fetch a decision's current state (including tallied quorum). */
    async getDecision(decisionId) {
        return this.#http.get(`/governance/decision/${encodeURIComponent(decisionId)}`);
    }
}
/** Authority chain gateway calls. */
export class AuthorityApi {
    #http;
    constructor(http) {
        this.#http = http;
    }
    /** Submit a validated authority chain for persistence. */
    async submitChain(chain) {
        return this.#http.post('/authority/chain', chain);
    }
    /** Fetch an authority chain by id. */
    async getChain(chainId) {
        return this.#http.get(`/authority/chain/${encodeURIComponent(chainId)}`);
    }
}
/** HonorGood and mission-economics calls. EXOCHAIN remains settlement authority. */
export class EconomyApi {
    #http;
    constructor(http) {
        this.#http = http;
    }
    async createMission(body) {
        return this.#http.post('/api/v1/economy/missions', body);
    }
    async getMission(id) {
        return this.#http.get(`/api/v1/economy/missions/${encodeURIComponent(id)}`);
    }
    async createContributionReceipt(body) {
        return this.#http.post('/api/v1/economy/contribution-receipts', body);
    }
    async createLegacyReceipt(body) {
        return this.#http.post('/api/v1/economy/legacy-receipts', body);
    }
    async getLegacyReceipt(id) {
        return this.#http.get(`/api/v1/economy/legacy-receipts/${encodeURIComponent(id)}`);
    }
    async createRuleset(body) {
        return this.#http.post('/api/v1/economy/rulesets', body);
    }
    async createContributionNode(body) {
        return this.#http.post('/api/v1/economy/contribution-nodes', body);
    }
    async createContributionOffer(body) {
        return this.#http.post('/api/v1/economy/contribution-offers', body);
    }
    async createContributionAcceptance(body) {
        return this.#http.post('/api/v1/economy/contribution-acceptances', body);
    }
    async createBailmentTerms(body) {
        return this.#http.post('/api/v1/economy/bailment-terms', body);
    }
    async createBailmentWrapper(body) {
        return this.#http.post('/api/v1/economy/bailment-wrappers', body);
    }
    async createAdoptionEvent(body) {
        return this.#http.post('/api/v1/economy/adoption-events', body);
    }
    async createUseEvent(body) {
        return this.#http.post('/api/v1/economy/use-events', body);
    }
    async createValueEvent(body) {
        return this.#http.post('/api/v1/economy/value-events', body);
    }
    async createMissionSettlement(body) {
        return this.#http.post('/api/v1/economy/mission-settlements', body);
    }
    async createAutomatedSettlement(body) {
        return this.#http.post('/api/v1/economy/automated-settlements', body);
    }
}
// -----------------------------------------------------------------------------
// Client
// -----------------------------------------------------------------------------
/** High-level client combining all domain APIs over a shared transport. */
export class ExochainClient {
    identity;
    consent;
    governance;
    authority;
    economy;
    #http;
    constructor(opts) {
        const transportOpts = {};
        if (opts.apiKey !== undefined)
            transportOpts.apiKey = opts.apiKey;
        if (opts.timeout !== undefined)
            transportOpts.timeout = opts.timeout;
        if (opts.fetch !== undefined)
            transportOpts.fetch = opts.fetch;
        this.#http = new HttpTransport(opts.baseUrl, transportOpts);
        this.identity = new IdentityApi(this.#http);
        this.consent = new ConsentApi(this.#http);
        this.governance = new GovernanceApi(this.#http);
        this.authority = new AuthorityApi(this.#http);
        this.economy = new EconomyApi(this.#http);
    }
    /** Gateway health probe. */
    async health() {
        return this.#http.health();
    }
}
//# sourceMappingURL=client.js.map