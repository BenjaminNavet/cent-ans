"""Validates the melee height advantage rules against their schema (SG4)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_crest.json").read_text(encoding="utf-8"))


def test_battle_crest_rules_match_schema() -> None:
    """`data/rules/battle_crest.json` matches `battle_crest_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "battle_crest_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_height_advantage_stays_moderate() -> None:
    """The height never more than halves nor doubles the melee damage."""
    rules = _rules()
    edge = rules["melee_per_m"] * rules["max_height_m"]
    assert 0.0 <= edge <= 0.5
