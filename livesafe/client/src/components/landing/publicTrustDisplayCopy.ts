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
    'Route status: public_claims_allowed=true and machine_state=public_trust_claims_allowed.',
  trustStripItems: [
    'Adapter-returned public trust state',
    'Source route: /api/trust/status',
    'Safety operations remain separate',
  ],
  underTheHoodHeading: 'Public trust display is route-authorized.',
  underTheHoodBody:
    'The landing page may show this EXOCHAIN public trust output because the trust-status route returned the authorized public-claims machine state. Medical, legal, custody, consent, and emergency decisions remain separate operational duties.',
  identityIdentifierLabel: 'did:exo DID',
  governanceCardTitle: 'Governed by authorized route state',
  governanceCardBody:
    'The public trust display is tied to the adapter-returned route state, not to proximity or marketing copy. If the route drops out of the authorized machine state, this section returns to inactive language.',
  footerStatus: 'Public trust display authorized by route state',
};

function readString(value: unknown): string | null {
  return typeof value === 'string' ? value : null;
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
  };
}

export function isAuthorizedPublicTrustRoute(status?: unknown): boolean {
  const routeStatus = normalizePublicTrustRouteContract(status);

  return (
    routeStatus?.public_claims_allowed === true &&
    routeStatus.machine_state === 'public_trust_claims_allowed' &&
    routeStatus.state === 'externally-verified' &&
    routeStatus.display_text === 'VERIFIED' &&
    routeStatus.runtime_adapter_state === 'verified' &&
    routeStatus.verified_runtime_adapter === true
  );
}

export function getLandingPublicTrustDisplayCopy(
  status?: unknown,
): LandingPublicTrustDisplayCopy {
  return isAuthorizedPublicTrustRoute(status) ? ACTIVE_COPY : INACTIVE_COPY;
}
