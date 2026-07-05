export interface TrustStatusPayloadOptions {
  exochainConnected: boolean;
  version: string;
  uptimeSeconds: number;
  generatedAt?: string;
  runtimeStatus?: {
    adapter_state: "not-wired" | "unverified" | "verified";
    surface_classification:
      | "exochain-core"
      | "core-runtime-adapter"
      | "adjacent-surface"
      | "imported-evidence"
      | "third-party-vendor";
    public_claims_allowed: boolean;
    can_read_exochain_core_state: boolean;
    can_write_exochain_core_state: boolean;
    wrapped_operations?: Array<
      | "getIdentity"
      | "registerIdentity"
      | "anchorAuditReceipt"
      | "anchorScan"
      | "anchorConsent"
      | "getPaceStatus"
      | "getPublicAdapterOutputAuthorization"
    >;
    disablement_path: string;
    source_basis: string[];
  };
  adapterOutputAuthorization?: {
    allowed: boolean;
    responseState: string;
    transportCalled: boolean;
    value: unknown;
  };
  productionTrustEvidence?: {
    evidence_state: "verified" | "blocked";
    production_health_verified: boolean;
    production_ready_verified: boolean;
    root_trust_bundle_verified: boolean;
    root_trust_bundle_id?: string | null;
    root_trust_ceremony_id?: string | null;
    root_trust_issuer_did?: string | null;
    verifier_commit?: string | null;
    verified_at?: string | null;
    reasons?: string[];
    non_blocking_observations?: string[];
  };
}

export interface TrustStatusPayload {
  state: "not-verified" | "externally-verified";
  badge_text: "AVC";
  icon: "lock-open" | "lock-check";
  color: "red" | "green";
  css_class: string;
  glow_class: string;
  display_text: "THIS IS NOT YET VERIFIED" | "VERIFIED";
  machine_state: "not_verified" | "public_trust_claims_allowed";
  api_surface: "api-response";
  exochain_connected: boolean;
  verified_runtime_adapter: boolean;
  runtime_adapter_state: "not-wired" | "unverified" | "verified";
  adapter_surface_classification:
    | "exochain-core"
    | "core-runtime-adapter"
    | "adjacent-surface"
    | "imported-evidence"
    | "third-party-vendor";
  runtime_adapter_operations: Array<
    | "getIdentity"
    | "registerIdentity"
    | "anchorAuditReceipt"
    | "anchorScan"
    | "anchorConsent"
    | "getPaceStatus"
    | "getPublicAdapterOutputAuthorization"
  >;
  adapter_disablement_path: string;
  exochain_production_evidence_state: "verified" | "blocked";
  exochain_production_health_verified: boolean;
  exochain_production_ready_verified: boolean;
  exochain_root_trust_bundle_verified: boolean;
  exochain_root_trust_bundle_id: string | null;
  exochain_root_trust_ceremony_id: string | null;
  exochain_root_trust_issuer_did: string | null;
  exochain_root_trust_verifier_commit: string | null;
  exochain_root_trust_verified_at: string | null;
  production_trust_observations: string[];
  production_trust_reasons: string[];
  internal_proof_complete: boolean;
  frost_genesis_complete: boolean;
  public_claims_allowed: boolean;
  public_claims_reason: string;
  public_adapter_output_authorization?: {
    schema: "livesafe.public_adapter_output_authorization.v1";
    subject: "livesafe.ai";
    audience: "https://livesafe.ai/api/trust/status";
    claims: string[];
    evidence_hash: string;
    receipt_id: string;
    proof_id: string;
    proof_ref: string;
    generated_at: string;
    valid_from: string;
    expires_at: string;
    proof_type: string;
    response_state: "permit";
    transport_called: true;
  };
  source_basis: string[];
  version: string;
  uptime_seconds: number;
  generated_at: string;
}

export function createTrustStatusPayload(
  options: TrustStatusPayloadOptions
): TrustStatusPayload;

export function buildLiveTrustStatusOptions(
  options: TrustStatusPayloadOptions & {
    adapter?: {
      getRuntimeStatus(): TrustStatusPayloadOptions["runtimeStatus"];
      getPublicAdapterOutputAuthorization(options: {
        currentAt: string;
        returnDecision: true;
      }): Promise<{
        allowed: boolean;
        responseState: string;
        transportCalled: boolean;
        value: unknown;
      }>;
    };
  }
): Promise<TrustStatusPayloadOptions>;

export function sendTrustStatusResponse(
  req: unknown,
  res: {
    status(code: number): {
      json(payload: TrustStatusPayload): unknown;
    };
  },
  options: TrustStatusPayloadOptions
): unknown;

export function sendLiveTrustStatusResponse(
  req: unknown,
  res: {
    status(code: number): {
      json(payload: TrustStatusPayload): unknown;
    };
  },
  options: TrustStatusPayloadOptions & {
    adapter?: {
      getRuntimeStatus(): TrustStatusPayloadOptions["runtimeStatus"];
      getPublicAdapterOutputAuthorization(options: {
        currentAt: string;
        returnDecision: true;
      }): Promise<{
        allowed: boolean;
        responseState: string;
        transportCalled: boolean;
        value: unknown;
      }>;
    };
  }
): Promise<unknown>;
