"""Validates the custom battle rules against their schema (NT2)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "custom_battle.json").read_text(encoding="utf-8")
    )


def test_budget_bounds_are_consistent() -> None:
    """12000 points by default (doubled with the unit cap), within the adjustable range."""
    rules = _rules()
    assert rules["default_budget"] == 12000
    assert rules["min_budget"] <= rules["default_budget"] <= rules["max_budget"]
    assert rules["default_fortification"] <= rules["max_fortification"]
