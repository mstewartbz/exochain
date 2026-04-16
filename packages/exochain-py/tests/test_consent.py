"""Tests for the BailmentBuilder and BailmentProposal."""

from __future__ import annotations

import pytest

from exochain import BailmentBuilder, ConsentError, Did

ALICE: Did = "did:exo:alice0001"
BOB: Did = "did:exo:bob0000002"


def test_builder_happy_path() -> None:
    """A fully-specified builder produces a frozen proposal with a hash id."""
    proposal = (
        BailmentBuilder(ALICE, BOB)
        .scope("read:health-record")
        .duration_hours(48)
        .build()
    )
    assert proposal.bailor == ALICE
    assert proposal.bailee == BOB
    assert proposal.scope == "read:health-record"
    assert proposal.duration_hours == 48
    # SHA-256 hex is 64 characters.
    assert len(proposal.proposal_id) == 64


def test_missing_scope_fails() -> None:
    """Calling `build` without setting a scope raises ConsentError."""
    builder = BailmentBuilder(ALICE, BOB).duration_hours(12)
    with pytest.raises(ConsentError):
        builder.build()


def test_empty_scope_rejected() -> None:
    """An empty or whitespace-only scope is rejected on set."""
    with pytest.raises(ConsentError):
        BailmentBuilder(ALICE, BOB).scope("")
    with pytest.raises(ConsentError):
        BailmentBuilder(ALICE, BOB).scope("   ")


def test_negative_duration_fails() -> None:
    """Non-positive durations are rejected."""
    with pytest.raises(ConsentError):
        BailmentBuilder(ALICE, BOB).scope("x").duration_hours(0)
    with pytest.raises(ConsentError):
        BailmentBuilder(ALICE, BOB).scope("x").duration_hours(-1)


def test_proposal_id_is_deterministic() -> None:
    """Two equivalent builders produce the same proposal_id."""
    a = BailmentBuilder(ALICE, BOB).scope("read").duration_hours(24).build()
    b = BailmentBuilder(ALICE, BOB).scope("read").duration_hours(24).build()
    assert a.proposal_id == b.proposal_id


def test_proposal_id_differs_for_different_inputs() -> None:
    """Distinct scopes or durations produce distinct ids."""
    base = BailmentBuilder(ALICE, BOB).scope("read").duration_hours(24).build()
    diff_scope = BailmentBuilder(ALICE, BOB).scope("write").duration_hours(24).build()
    diff_duration = BailmentBuilder(ALICE, BOB).scope("read").duration_hours(48).build()
    assert base.proposal_id != diff_scope.proposal_id
    assert base.proposal_id != diff_duration.proposal_id


def test_proposal_is_frozen() -> None:
    """Proposal fields cannot be mutated after construction."""
    proposal = BailmentBuilder(ALICE, BOB).scope("x").build()
    with pytest.raises((TypeError, ValueError)):
        proposal.scope = "other"
