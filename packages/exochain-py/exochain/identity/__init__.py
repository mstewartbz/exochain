"""Identity primitives: DID validation and Ed25519 keypair management."""

from __future__ import annotations

from .did import is_did, validate_did
from .keypair import Identity, derive_did

__all__ = ["Identity", "derive_did", "is_did", "validate_did"]
