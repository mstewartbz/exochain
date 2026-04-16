"""Typed primitives shared across the EXOCHAIN SDK.

These are Pydantic v2 models (or branded string aliases) that form the wire
contract between the SDK and the fabric. Where appropriate, models are frozen
so they can be safely used as dictionary keys and passed around by value.
"""

from __future__ import annotations

from typing import Annotated, Literal

from pydantic import BaseModel, ConfigDict, StringConstraints

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


__all__ = [
    "Did",
    "Hash256Hex",
    "Outcome",
    "QuorumResult",
    "TrustReceipt",
]
