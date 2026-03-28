"""Public-key registry for decision.forum custody verification.

We avoid storing secret keys.
A simple JSON mapping of actor_id -> public_key_b64 is enough for MVP.

Layout (default):
.decision_forum/keys.json
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, Optional


@dataclass
class KeyRegistry:
    root: Path = Path(".decision_forum")

    @property
    def path(self) -> Path:
        return self.root / "keys.json"

    def init(self) -> None:
        self.root.mkdir(parents=True, exist_ok=True)
        if not self.path.exists():
            self.path.write_text(json.dumps({}, indent=2) + "\n", encoding="utf-8")

    def load(self) -> Dict[str, str]:
        self.init()
        try:
            return json.loads(self.path.read_text(encoding="utf-8") or "{}")
        except Exception:
            return {}

    def get(self, actor_id: str) -> Optional[str]:
        return self.load().get(actor_id)

    def register(self, actor_id: str, public_key_b64: str) -> None:
        data = self.load()
        data[actor_id] = public_key_b64
        self.path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
