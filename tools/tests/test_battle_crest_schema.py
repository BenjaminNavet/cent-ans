"""Validates the melee height advantage rules against their schema (SG4)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_crest.json").read_text(encoding="utf-8")
    )


def test_height_advantage_stays_moderate() -> None:
    """The height never more than halves nor doubles the melee damage."""
    rules = _rules()
    edge = rules["melee_per_m"] * (rules["max_height_m"] - rules["min_height_m"])
    assert 0.0 <= edge <= 0.5
