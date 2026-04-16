"""EXOCHAIN SDK for Python.

EXOCHAIN is a constitutional governance fabric for AI agents and data
sovereignty. This package is the pure-Python SDK — Ed25519 identity,
bailment consent, governance decisions, authority chains, and an async
HTTP client.

Quick start:

    >>> from exochain import Identity
    >>> alice = Identity.generate("alice")
    >>> alice.did.startswith("did:exo:")
    True
    >>> sig = alice.sign(b"hello")
    >>> Identity.verify(alice.public_key_hex, b"hello", sig)
    True

See :class:`ExochainClient` for the async fabric client.
"""

from __future__ import annotations

from .authority.chain import AuthorityChainBuilder, ChainLink, ValidatedChain
from .client import ExochainClient
from .consent.bailment import BailmentBuilder, BailmentProposal
from .crypto.hash import sha256, sha256_hex
from .errors import (
    AuthorityError,
    ConsentError,
    CryptoError,
    ExochainError,
    GovernanceError,
    IdentityError,
    KernelError,
    TransportError,
)
from .governance.decision import Decision, DecisionBuilder, DecisionStatus
from .governance.vote import Vote, VoteChoice
from .identity.did import is_did, validate_did
from .identity.keypair import Identity
from .transport.http import HttpTransport
from .types import Did, Hash256Hex, QuorumResult, TrustReceipt

__version__ = "0.1.0"

__all__ = [
    "AuthorityChainBuilder",
    "AuthorityError",
    "BailmentBuilder",
    "BailmentProposal",
    "ChainLink",
    "ConsentError",
    "CryptoError",
    "Decision",
    "DecisionBuilder",
    "DecisionStatus",
    "Did",
    "ExochainClient",
    "ExochainError",
    "GovernanceError",
    "Hash256Hex",
    "HttpTransport",
    "Identity",
    "IdentityError",
    "KernelError",
    "QuorumResult",
    "TransportError",
    "TrustReceipt",
    "ValidatedChain",
    "Vote",
    "VoteChoice",
    "__version__",
    "is_did",
    "sha256",
    "sha256_hex",
    "validate_did",
]
