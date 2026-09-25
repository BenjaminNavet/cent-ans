"""Validates the battle water and roads rules against their schema (lot EP3)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_battle_water_rules_match_schema() -> None:
    """``data/rules/battle_water.json`` matches ``battle_water_rules.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "battle_water_rules.schema.json").read_text(encoding="utf-8")
    )
    document = json.loads(
        (DATA / "rules" / "battle_water.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
