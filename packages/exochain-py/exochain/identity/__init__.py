"""Identity primitives: DID validation and Ed25519 keypair management."""

from __future__ import annotations

from .did import is_did, validate_did
from .keypair import Identity

__all__ = ["Identity", "is_did", "validate_did"]
