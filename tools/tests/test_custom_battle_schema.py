"""Validates the custom battle rules against their schema (NT2)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "custom_battle.json").read_text(encoding="utf-8")
    )


def test_custom_battle_rules_match_schema() -> None:
    """`data/rules/custom_battle.json` matches `custom_battle_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "custom_battle_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_budget_bounds_are_consistent() -> None:
    """Spec NT2: 6000 points by default, within the adjustable range."""
    rules = _rules()
    assert rules["default_budget"] == 6000
    assert rules["min_budget"] <= rules["default_budget"] <= rules["max_budget"]
    assert rules["default_fortification"] <= rules["max_fortification"]
