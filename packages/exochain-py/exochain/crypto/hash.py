"""SHA-256 helpers backed by :mod:`hashlib`."""

from __future__ import annotations

from hashlib import sha256 as _sha256


def sha256(data: bytes) -> bytes:
    """Return the raw 32-byte SHA-256 digest of ``data``."""
    return _sha256(bytes(data)).digest()


def sha256_hex(data: bytes) -> str:
    """Return the 64-character lowercase hex SHA-256 digest of ``data``."""
    return _sha256(bytes(data)).hexdigest()


__all__ = ["sha256", "sha256_hex"]
