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

export interface PublicTrustRouteContract {
  state: string | null;
  display_text: string | null;
  machine_state: string | null;
  public_claims_allowed: boolean;
  runtime_adapter_state: string | null;
  verified_runtime_adapter: boolean;
  public_adapter_output_authorization: PublicAdapterOutputAuthorizationMetadata | null;
}

export interface PublicAdapterOutputAuthorizationMetadata {
  schema: string | null;
  subject: string | null;
  audience: string | null;
  claims: readonly string[];
  evidence_hash: string | null;
  receipt_id: string | null;
  proof_id: string | null;
  proof_ref: string | null;
  generated_at: string | null;
  valid_from: string | null;
  expires_at: string | null;
  proof_type: string | null;
  response_state: string | null;
  transport_called: boolean;
}

export interface LandingPublicTrustDisplayCopy {
  trustBearingClaimsVisible: boolean;
  machineState: 'not_verified' | 'public_trust_claims_allowed';
  trustStripLead: string;
  trustStripDetail: string;
  trustStripItems: readonly string[];
  underTheHoodHeading: string;
  underTheHoodBody: string;
  identityIdentifierLabel: string;
  governanceCardTitle: string;
  governanceCardBody: string;
  footerStatus: string;
}

const INACTIVE_COPY: LandingPublicTrustDisplayCopy = {
  trustBearingClaimsVisible: false,
  machineState: 'not_verified',
  trustStripLead: 'EXOCHAIN public trust copy inactive',
  trustStripDetail:
    'The public route has not authorized public EXOCHAIN trust claims.',
  trustStripItems: [
    'Safety app copy only',
    'Adapter proof required',
    'No public trust claim',
  ],
  underTheHoodHeading: 'Trust claims stay off until the route allows them.',
  underTheHoodBody:
    'LiveSafe can explain its safety workflow here, but public EXOCHAIN trust-bearing claims remain inactive until /api/trust/status returns the authorized public-claims machine state.',
  identityIdentifierLabel: 'local identifier',
  governanceCardTitle: 'Governance claims stay gated',
  governanceCardBody:
    'This page does not claim EXOCHAIN enforcement while the public trust route is inactive. The safety workflow remains an app-level design until adapter output allows public trust copy.',
  footerStatus: 'Public trust display inactive',
};

const ACTIVE_COPY: LandingPublicTrustDisplayCopy = {
  trustBearingClaimsVisible: true,
  machineState: 'public_trust_claims_allowed',
  trustStripLead: 'EXOCHAIN public trust output authorized',
  trustStripDetail:
    'Route status: public_claims_allowed=true and machine_state=public_trust_claims_allowed; public_adapter_output_authorization.response_state=permit.',
  trustStripItems: [
    'Proof-bearing adapter output',
    'Source route: /api/trust/status',
    'Safety operations remain separate',
  ],
  underTheHoodHeading: 'Public trust display is route-authorized.',
  underTheHoodBody:
    'The landing page may show this EXOCHAIN public trust output because the trust-status route returned the authorized public-claims machine state with proof-bearing public adapter-output metadata. Medical, legal, custody, consent, and emergency decisions remain separate operational duties.',
  identityIdentifierLabel: 'did:exo DID',
  governanceCardTitle: 'Governed by authorized route state',
  governanceCardBody:
    'The public trust display is tied to the adapter-returned route state, not to proximity or marketing copy. If the route drops out of the authorized machine state, this section returns to inactive language.',
  footerStatus: 'Public trust display authorized by route state',
};

function readString(value: unknown): string | null {
  return typeof value === 'string' ? value : null;
}

function readStringArray(value: unknown): readonly string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
    ? value
    : [];
}

function normalizePublicAdapterOutputAuthorization(
  authorization: unknown,
): PublicAdapterOutputAuthorizationMetadata | null {
  if (!authorization || typeof authorization !== 'object' || Array.isArray(authorization)) {
    return null;
  }

  const record = authorization as Record<string, unknown>;

  return {
    schema: readString(record.schema),
    subject: readString(record.subject),
    audience: readString(record.audience),
    claims: readStringArray(record.claims),
    evidence_hash: readString(record.evidence_hash),
    receipt_id: readString(record.receipt_id),
    proof_id: readString(record.proof_id),
    proof_ref: readString(record.proof_ref),
    generated_at: readString(record.generated_at),
    valid_from: readString(record.valid_from),
    expires_at: readString(record.expires_at),
    proof_type: readString(record.proof_type),
    response_state: readString(record.response_state),
    transport_called: record.transport_called === true,
  };
}

export function normalizePublicTrustRouteContract(
  status?: unknown,
): PublicTrustRouteContract | null {
  if (!status || typeof status !== 'object' || Array.isArray(status)) {
    return null;
  }

  const record = status as Record<string, unknown>;

  return {
    state: readString(record.state),
    display_text: readString(record.display_text),
    machine_state: readString(record.machine_state),
    public_claims_allowed: record.public_claims_allowed === true,
    runtime_adapter_state: readString(record.runtime_adapter_state),
    verified_runtime_adapter: record.verified_runtime_adapter === true,
    public_adapter_output_authorization: normalizePublicAdapterOutputAuthorization(
      record.public_adapter_output_authorization,
    ),
  };
}

const REQUIRED_PUBLIC_ADAPTER_OUTPUT_CLAIMS = [
  'livesafe_public_trust_status',
  'exochain_production_evidence_verified',
  'livesafe_runtime_adapter_verified',
] as const;

function isNonEmptyString(value: string | null): value is string {
  return typeof value === 'string' && value.length > 0;
}

function hasRequiredAdapterOutputClaims(claims: readonly string[]): boolean {
  return REQUIRED_PUBLIC_ADAPTER_OUTPUT_CLAIMS.every((claim) =>
    claims.includes(claim),
  );
}

function hasAuthorizedPublicAdapterOutputMetadata(
  authorization: PublicAdapterOutputAuthorizationMetadata | null,
): boolean {
  return (
    authorization?.schema === 'livesafe.public_adapter_output_authorization.v1' &&
    authorization.subject === 'livesafe.ai' &&
    authorization.audience === 'https://livesafe.ai/api/trust/status' &&
    hasRequiredAdapterOutputClaims(authorization.claims) &&
    isNonEmptyString(authorization.evidence_hash) &&
    authorization.evidence_hash.startsWith('sha256:') &&
    isNonEmptyString(authorization.receipt_id) &&
    isNonEmptyString(authorization.proof_id) &&
    isNonEmptyString(authorization.proof_ref) &&
    isNonEmptyString(authorization.generated_at) &&
    isNonEmptyString(authorization.valid_from) &&
    isNonEmptyString(authorization.expires_at) &&
    authorization.proof_type === 'ed25519-public-adapter-output-authorization' &&
    authorization.response_state === 'permit' &&
    authorization.transport_called === true
  );
}

export function isAuthorizedPublicTrustRoute(status?: unknown): boolean {
  const routeStatus = normalizePublicTrustRouteContract(status);

  return (
    routeStatus?.public_claims_allowed === true &&
    routeStatus.machine_state === 'public_trust_claims_allowed' &&
    routeStatus.state === 'externally-verified' &&
    routeStatus.display_text === 'VERIFIED' &&
    routeStatus.runtime_adapter_state === 'verified' &&
    routeStatus.verified_runtime_adapter === true &&
    hasAuthorizedPublicAdapterOutputMetadata(
      routeStatus.public_adapter_output_authorization,
    )
  );
}

export function getLandingPublicTrustDisplayCopy(
  status?: unknown,
): LandingPublicTrustDisplayCopy {
  return isAuthorizedPublicTrustRoute(status) ? ACTIVE_COPY : INACTIVE_COPY;
}
