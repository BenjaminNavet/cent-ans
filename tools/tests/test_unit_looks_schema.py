"""Validates data/fx/unit_looks.json against unit_looks.schema.json (lot OMR R5)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_unit_looks_match_schema() -> None:
    """The unit looks file exists and matches its schema."""
    schema = _load("schemas/unit_looks.schema.json")
    looks = _load("fx/unit_looks.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(looks), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_unit_looks_name_real_unit_types() -> None:
    """Every unit type with a look exists in data/unit_types."""
    unit_types = {path.stem for path in (DATA / "unit_types").glob("unit_*.json")}
    looks = _load("fx/unit_looks.json")["looks"]
    assert set(looks) <= unit_types, set(looks) - unit_types
