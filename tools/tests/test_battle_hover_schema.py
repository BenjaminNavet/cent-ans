"""Validates the battle path preview and hover cursor rules against their schema (CB-M2)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_hover.json").read_text(encoding="utf-8"))


def test_battle_hover_rules_match_schema() -> None:
    """`data/rules/battle_hover.json` matches `battle_hover_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "battle_hover_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_preview_throttle_matches_the_spec() -> None:
    """Spec CB-M: recompute beyond 5 m, at most 10 per second, one path beyond 6 units."""
    preview = _rules()["preview"]
    assert preview["recompute_distance_m"] == 5.0
    assert preview["max_recomputes_per_s"] == 10.0
    assert preview["max_individual_paths"] == 6
