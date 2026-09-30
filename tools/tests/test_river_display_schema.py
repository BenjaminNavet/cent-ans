"""Validates data/map/river_display.json against its schema (lot RC3)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_river_display_matches_schema() -> None:
    """The campaign river display file matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "river_display.schema.json").read_text(encoding="utf-8")
    )
    display = json.loads(
        (DATA / "map" / "river_display.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(display), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
