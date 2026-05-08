/**
 * High-level {@link ExochainClient} — the one-stop entry point for talking to
 * an `exo-gateway` instance. Each domain API (identity, consent, governance,
 * authority) wraps the shared {@link HttpTransport}.
 */

import { HttpTransport } from './transport/http.js';
import type {
  AutomatedSettlementRequest,
  EconomyObjectResponse,
  HealthResponse,
  Did,
  Hash256,
  MissionSettlementRequest,
  QuorumResult,
} from './types.js';

/** Options for constructing an {@link ExochainClient}. */
export interface ExochainClientOptions {
  readonly baseUrl: string;
  readonly apiKey?: string;
  readonly timeout?: number;
  readonly fetch?: typeof fetch;
}

// -----------------------------------------------------------------------------
// Domain API surfaces
// -----------------------------------------------------------------------------

/** Identity-related gateway calls. */
export class IdentityApi {
  readonly #http: HttpTransport;
  constructor(http: HttpTransport) {
    this.#http = http;
  }

  /** Resolve a DID to its DID document via `GET /identity/did/{did}`. */
  public async resolve(did: Did): Promise<unknown> {
    return this.#http.get(`/identity/did/${encodeURIComponent(did)}`);
  }

  /** Register a DID document via `POST /identity/did`. */
  public async register(document: unknown): Promise<{ did: Did }> {
    return this.#http.post<{ did: Did }>('/identity/did', document);
  }
}

/** Consent / bailment gateway calls. */
export class ConsentApi {
  readonly #http: HttpTransport;
  constructor(http: HttpTransport) {
    this.#http = http;
  }

  /** Submit a bailment proposal for processing. */
  public async proposeBailment(body: unknown): Promise<{ proposalId: Hash256 }> {
    return this.#http.post<{ proposalId: Hash256 }>('/consent/bailment', body);
  }

  /** Fetch a bailment proposal by its content-addressed ID. */
  public async getBailment(proposalId: Hash256): Promise<unknown> {
    return this.#http.get(`/consent/bailment/${encodeURIComponent(proposalId)}`);
  }
}

/** Governance gateway calls. */
export class GovernanceApi {
  readonly #http: HttpTransport;
  constructor(http: HttpTransport) {
    this.#http = http;
  }

  /** Create a decision via `POST /governance/decision`. */
  public async createDecision(body: unknown): Promise<{ decisionId: Hash256 }> {
    return this.#http.post<{ decisionId: Hash256 }>('/governance/decision', body);
  }

  /** Cast a vote on an existing decision. */
  public async castVote(decisionId: Hash256, body: unknown): Promise<void> {
    await this.#http.post<void>(
      `/governance/decision/${encodeURIComponent(decisionId)}/vote`,
      body,
    );
  }

  /** Fetch a decision's current state (including tallied quorum). */
  public async getDecision(
    decisionId: Hash256,
  ): Promise<{ decisionId: Hash256; status: string; quorum?: QuorumResult }> {
    return this.#http.get(`/governance/decision/${encodeURIComponent(decisionId)}`);
  }
}

/** Authority chain gateway calls. */
export class AuthorityApi {
  readonly #http: HttpTransport;
  constructor(http: HttpTransport) {
    this.#http = http;
  }

  /** Submit a validated authority chain for persistence. */
  public async submitChain(chain: unknown): Promise<{ chainId: Hash256 }> {
    return this.#http.post<{ chainId: Hash256 }>('/authority/chain', chain);
  }

  /** Fetch an authority chain by id. */
  public async getChain(chainId: Hash256): Promise<unknown> {
    return this.#http.get(`/authority/chain/${encodeURIComponent(chainId)}`);
  }
}

/** HonorGood and mission-economics calls. EXOCHAIN remains settlement authority. */
export class EconomyApi {
  readonly #http: HttpTransport;
  constructor(http: HttpTransport) {
    this.#http = http;
  }

  public async createMission<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/missions', body);
  }

  public async getMission<T = unknown>(id: Hash256): Promise<T> {
    return this.#http.get<T>(`/api/v1/economy/missions/${encodeURIComponent(id)}`);
  }

  public async createContributionReceipt<T = unknown>(
    body: T,
  ): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>(
      '/api/v1/economy/contribution-receipts',
      body,
    );
  }

  public async createLegacyReceipt<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/legacy-receipts', body);
  }

  public async getLegacyReceipt<T = unknown>(id: Hash256): Promise<T> {
    return this.#http.get<T>(`/api/v1/economy/legacy-receipts/${encodeURIComponent(id)}`);
  }

  public async createRuleset<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/rulesets', body);
  }

  public async createContributionNode<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/contribution-nodes', body);
  }

  public async createContributionOffer<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/contribution-offers', body);
  }

  public async createContributionAcceptance<T = unknown>(
    body: T,
  ): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>(
      '/api/v1/economy/contribution-acceptances',
      body,
    );
  }

  public async createBailmentTerms<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/bailment-terms', body);
  }

  public async createBailmentWrapper<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/bailment-wrappers', body);
  }

  public async createAdoptionEvent<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/adoption-events', body);
  }

  public async createUseEvent<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/use-events', body);
  }

  public async createValueEvent<T = unknown>(body: T): Promise<EconomyObjectResponse<T>> {
    return this.#http.post<EconomyObjectResponse<T>>('/api/v1/economy/value-events', body);
  }

  public async createMissionSettlement(
    body: MissionSettlementRequest,
  ): Promise<EconomyObjectResponse> {
    return this.#http.post<EconomyObjectResponse>('/api/v1/economy/mission-settlements', body);
  }

  public async createAutomatedSettlement(
    body: AutomatedSettlementRequest,
  ): Promise<EconomyObjectResponse> {
    return this.#http.post<EconomyObjectResponse>('/api/v1/economy/automated-settlements', body);
  }
}

// -----------------------------------------------------------------------------
// Client
// -----------------------------------------------------------------------------

/** High-level client combining all domain APIs over a shared transport. */
export class ExochainClient {
  public readonly identity: IdentityApi;
  public readonly consent: ConsentApi;
  public readonly governance: GovernanceApi;
  public readonly authority: AuthorityApi;
  public readonly economy: EconomyApi;
  readonly #http: HttpTransport;

  constructor(opts: ExochainClientOptions) {
    const transportOpts: {
      apiKey?: string;
      timeout?: number;
      fetch?: typeof fetch;
    } = {};
    if (opts.apiKey !== undefined) transportOpts.apiKey = opts.apiKey;
    if (opts.timeout !== undefined) transportOpts.timeout = opts.timeout;
    if (opts.fetch !== undefined) transportOpts.fetch = opts.fetch;
    this.#http = new HttpTransport(opts.baseUrl, transportOpts);
    this.identity = new IdentityApi(this.#http);
    this.consent = new ConsentApi(this.#http);
    this.governance = new GovernanceApi(this.#http);
    this.authority = new AuthorityApi(this.#http);
    this.economy = new EconomyApi(this.#http);
  }

  /** Gateway health probe. */
  public async health(): Promise<HealthResponse> {
    return this.#http.health();
  }
}
