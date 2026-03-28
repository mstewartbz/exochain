"""Small utility helpers for the MVP.

This file intentionally stays dependency-light.
"""

from __future__ import annotations

from pathlib import Path
from typing import Any, Dict

import yaml


def load_upk(path: str | Path = "upk.yaml") -> Dict[str, Any]:
    """Load the Root Prompt Kernel (UPK) YAML."""
    p = Path(path)
    if not p.exists():
        raise FileNotFoundError(f"UPK not found at {p.resolve()}")
    return yaml.safe_load(p.read_text(encoding="utf-8"))


def retrieve_memory_context(_prompt: str) -> str:
    """Placeholder for vector/graph memory retrieval.

    MVP returns empty string to keep the CLI usable without external services.
    """

    return ""
