"""Validates the horse wait rules against their schema (EQ7)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_horse_wait.json").read_text(encoding="utf-8")
    )


def test_committed_horse_is_closer_than_its_foot() -> None:
    """A horse stops being held only once nearer its target than the foot must be."""
    rules = _rules()
    assert rules["committed_m"] <= rules["foot_close_m"]
