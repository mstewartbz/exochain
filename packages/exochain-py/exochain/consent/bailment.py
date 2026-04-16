"""Bailment consent proposals.

A *bailment* is a scoped, time-bounded delegation of custody from a bailor
to a bailee. :class:`BailmentBuilder` assembles and validates a proposal,
producing an immutable :class:`BailmentProposal` with a deterministic id.
"""

from __future__ import annotations

from hashlib import sha256

from pydantic import BaseModel, ConfigDict

from ..errors import ConsentError
from ..types import Did, Hash256Hex


class BailmentProposal(BaseModel):
    """A validated, immutable bailment proposal."""

    model_config = ConfigDict(frozen=True)

    proposal_id: Hash256Hex
    bailor: Did
    bailee: Did
    scope: str
    duration_hours: int


class BailmentBuilder:
    """Fluent builder for a :class:`BailmentProposal`.

    Example:
        >>> proposal = (
        ...     BailmentBuilder("did:exo:alice", "did:exo:bob")
        ...     .scope("read:medical-records")
        ...     .duration_hours(48)
        ...     .build()
        ... )
    """

    def __init__(self, bailor: Did, bailee: Did) -> None:
        self._bailor: Did = bailor
        self._bailee: Did = bailee
        self._scope: str | None = None
        self._duration_hours: int = 24

    def scope(self, scope: str) -> BailmentBuilder:
        """Set the scope descriptor for this bailment."""
        if not isinstance(scope, str) or not scope.strip():
            raise ConsentError("scope cannot be empty")
        self._scope = scope
        return self

    def duration_hours(self, hours: int) -> BailmentBuilder:
        """Set the duration of the bailment in hours."""
        if not isinstance(hours, int) or isinstance(hours, bool) or hours <= 0:
            raise ConsentError("duration_hours must be a positive integer")
        self._duration_hours = hours
        return self

    def build(self) -> BailmentProposal:
        """Validate and produce a :class:`BailmentProposal`.

        The ``proposal_id`` is the hex-encoded SHA-256 of a canonical payload
        containing the bailor, bailee, scope, and duration.
        """
        if self._scope is None:
            raise ConsentError("scope is required")

        payload = f"{self._bailor}|{self._bailee}|{self._scope}|{self._duration_hours}"
        proposal_id: Hash256Hex = sha256(payload.encode("utf-8")).hexdigest()
        return BailmentProposal(
            proposal_id=proposal_id,
            bailor=self._bailor,
            bailee=self._bailee,
            scope=self._scope,
            duration_hours=self._duration_hours,
        )


__all__ = ["BailmentBuilder", "BailmentProposal"]
