"""Validates data/rules/economy.json against economy_rules.schema.json (lot EQ1)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_economy_rules_match_schema() -> None:
    """The economy rules file exists and matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "economy_rules.schema.json").read_text(encoding="utf-8")
    )
    rules = json.loads((DATA / "rules" / "economy.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
