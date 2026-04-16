"""Exception hierarchy for the EXOCHAIN SDK.

All SDK-specific exceptions derive from :class:`ExochainError` so callers can
catch the base type when they do not care about the specific failure mode.
"""

from __future__ import annotations


class ExochainError(Exception):
    """Base exception for all EXOCHAIN SDK errors."""


class IdentityError(ExochainError):
    """Raised for identity / DID / keypair failures."""


class ConsentError(ExochainError):
    """Raised for consent-related failures (e.g. bailment validation)."""


class GovernanceError(ExochainError):
    """Raised for governance failures (e.g. decision / voting errors)."""


class AuthorityError(ExochainError):
    """Raised for authority chain validation failures."""


class KernelError(ExochainError):
    """Raised for constitutional kernel errors."""


class CryptoError(ExochainError):
    """Raised for cryptographic failures."""


class TransportError(ExochainError):
    """Raised for HTTP / network transport failures."""


__all__ = [
    "AuthorityError",
    "ConsentError",
    "CryptoError",
    "ExochainError",
    "GovernanceError",
    "IdentityError",
    "KernelError",
    "TransportError",
]
