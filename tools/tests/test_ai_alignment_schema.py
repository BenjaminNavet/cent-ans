"""Validates data/ai/alignment.json against ai_alignment.schema.json (G4)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_a_grudge_is_a_negative_sum() -> None:
    """A grievance is a sum of negative opinion modifiers, never a positive one."""
    tuning = json.loads((DATA / "ai" / "alignment.json").read_text(encoding="utf-8"))
    assert tuning["grievance"]["grudge"] < 0
