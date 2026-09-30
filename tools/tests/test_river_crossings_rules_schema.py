"""Validates data/rules/river_crossings.json against its schema (lot RC3a)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_river_crossings_rules_match_schema() -> None:
    """The river crossing rules file matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "river_crossings_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    rules = json.loads(
        (DATA / "rules" / "river_crossings.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
