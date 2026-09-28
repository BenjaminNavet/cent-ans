"""Validates data/ai/feudal.json against ai_feudal.schema.json (lot FE5)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_ai_feudal_matches_schema() -> None:
    """The feudal AI weights exist and match their schema."""
    schema = json.loads(
        (DATA / "schemas" / "ai_feudal.schema.json").read_text(encoding="utf-8")
    )
    weights = json.loads((DATA / "ai" / "feudal.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(weights), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
