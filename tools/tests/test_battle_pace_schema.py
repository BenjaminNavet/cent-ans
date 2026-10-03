"""Validates the fighting pace rules against their schema (A6-L13)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_battle_pace_rules_match_schema() -> None:
    """`data/rules/battle_pace.json` matches `battle_pace_rules.schema.json`."""
    rules = json.loads((DATA / "rules" / "battle_pace.json").read_text(encoding="utf-8"))
    schema = json.loads(
        (DATA / "schemas" / "battle_pace_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
