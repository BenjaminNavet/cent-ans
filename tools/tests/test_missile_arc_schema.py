"""Validates data/rules/missile_arc.json against missile_arc_rules.schema.json (lot R4)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_missile_arc_rules_match_schema() -> None:
    """The indirect fire rules file exists and matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "missile_arc_rules.schema.json").read_text(encoding="utf-8")
    )
    rules = json.loads((DATA / "rules" / "missile_arc.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
    assert rules["remembered_accuracy"] <= rules["indirect_accuracy"]
