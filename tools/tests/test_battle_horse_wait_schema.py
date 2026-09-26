"""Validates the horse wait rules against their schema (EQ7)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_horse_wait.json").read_text(encoding="utf-8")
    )


def test_battle_horse_wait_rules_match_schema() -> None:
    """`data/rules/battle_horse_wait.json` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "battle_horse_wait_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_committed_horse_is_closer_than_its_foot() -> None:
    """A horse stops being held only once nearer its target than the foot must be."""
    rules = _rules()
    assert rules["committed_m"] <= rules["foot_close_m"]
