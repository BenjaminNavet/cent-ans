"""Validates data/rules/missile_arc.json against missile_arc_rules.schema.json (lot R4)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_missile_arc_rules_match_schema() -> None:
    """The indirect fire rules file exists and matches its schema."""
    rules = json.loads(
        (DATA / "rules" / "missile_arc.json").read_text(encoding="utf-8")
    )
    assert rules["remembered_accuracy"] <= rules["indirect_accuracy"]
