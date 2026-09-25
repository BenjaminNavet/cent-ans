"""Validates data/rules/population.json against population_rules.schema.json (lot E2)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_population_rules_match_schema() -> None:
    """The public order rules file exists and matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "population_rules.schema.json").read_text(encoding="utf-8")
    )
    rules = json.loads((DATA / "rules" / "population.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
