"""Validates data/ai/diplomacy.json against ai_diplomacy.schema.json (G5)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_a_pretender_alone_needs_more_power() -> None:
    """A pretender with neither allies nor border needs a higher ratio."""
    tuning = json.loads((DATA / "ai" / "diplomacy.json").read_text(encoding="utf-8"))
    war = tuning["war"]
    assert war["pretender_ratio_alone"] >= war["pretender_ratio"]
