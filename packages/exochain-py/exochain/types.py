# Copyright 2026 Exochain Foundation
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at:
#
#     https://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.
#
# SPDX-License-Identifier: Apache-2.0

"""Typed primitives shared across the EXOCHAIN SDK.

These are Pydantic v2 models (or branded string aliases) that form the wire
contract between the SDK and the fabric. Where appropriate, models are frozen
so they can be safely used as dictionary keys and passed around by value.
"""

from __future__ import annotations

from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, Field, StrictBool, StringConstraints

# A DID on the exo network: "did:exo:" followed by a base58-alphanumeric suffix.
Did = Annotated[
    str,
    StringConstraints(pattern=r"^did:exo:[A-Za-z0-9]+$", min_length=10),
]

# A lowercase hex-encoded SHA-256 digest (64 characters).
Hash256Hex = Annotated[
    str,
    StringConstraints(pattern=r"^[0-9a-f]{64}$"),
]

# Allowed gatekeeper outcomes when evaluating a proposed action.
Outcome = Literal["permitted", "denied", "escalated"]


class TrustReceipt(BaseModel):
    """An immutable receipt attesting to a constitutional decision.

    Returned by the gatekeeper whenever an action is evaluated against the
    fabric's policy surface.
    """

    model_config = ConfigDict(frozen=True)

    receipt_hash: Hash256Hex
    actor_did: Did
    action_type: str
    outcome: Outcome
    timestamp_ms: int


class QuorumResult(BaseModel):
    """Result of a quorum check on a :class:`~exochain.governance.Decision`."""

    model_config = ConfigDict(frozen=True)

    met: bool
    threshold: int
    total_votes: int
    approvals: int
    rejections: int
    abstentions: int


class ExochainAvcDiscoveryRoutes(BaseModel):
    """AVC route paths from the public EXOCHAIN discovery document."""

    model_config = ConfigDict(frozen=True, populate_by_name=True)

    issue: str
    validate_route: str = Field(alias="validate")
    receipts_emit: str
    receipts_get: str
    protocol: str


class ExochainDiscoveryRoutes(BaseModel):
    """Public route paths from the canonical EXOCHAIN node."""

    model_config = ConfigDict(frozen=True)

    health: str
    ready: str
    avc: ExochainAvcDiscoveryRoutes


class ExochainSdkDiscovery(BaseModel):
    """SDK package locations advertised by the canonical EXOCHAIN node."""

    model_config = ConfigDict(frozen=True)

    rust: str
    typescript: str
    python: str


class ExochainMcpDiscovery(BaseModel):
    """MCP capability metadata; public_transport false means discovery only."""

    model_config = ConfigDict(frozen=True)

    public_transport: StrictBool
    transports: tuple[str, ...]
    capabilities: tuple[str, ...]


class ExochainDiscoveryResponse(BaseModel):
    """Public EXOCHAIN discovery document."""

    model_config = ConfigDict(frozen=True)

    base_url: str
    routes: ExochainDiscoveryRoutes
    sdk: ExochainSdkDiscovery
    mcp: ExochainMcpDiscovery


__all__ = [
    "Did",
    "ExochainAvcDiscoveryRoutes",
    "ExochainDiscoveryResponse",
    "ExochainDiscoveryRoutes",
    "ExochainMcpDiscovery",
    "ExochainSdkDiscovery",
    "Hash256Hex",
    "Outcome",
    "QuorumResult",
    "TrustReceipt",
]
