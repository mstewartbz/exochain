"""Vote primitives for governance decisions."""

from __future__ import annotations

from enum import StrEnum

from pydantic import BaseModel, ConfigDict

from ..types import Did


class VoteChoice(StrEnum):
    """A voter's choice on a governance decision."""

    APPROVE = "approve"
    REJECT = "reject"
    ABSTAIN = "abstain"


class Vote(BaseModel):
    """A single vote cast on a :class:`~exochain.governance.Decision`."""

    model_config = ConfigDict(frozen=True)

    voter: Did
    choice: VoteChoice
    rationale: str | None = None


__all__ = ["Vote", "VoteChoice"]
