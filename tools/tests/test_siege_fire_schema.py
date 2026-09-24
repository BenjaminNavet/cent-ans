"""Validates the siege fire rules and visual parameters against their schemas (lot S2)."""

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


@pytest.mark.parametrize(
    ("schema_name", "data_path"),
    [
        ("siege_fire_rules.schema.json", Path("rules") / "siege_fire.json"),
        ("fx_siege_fire.schema.json", Path("fx") / "siege_fire.json"),
    ],
)
def test_siege_fire_files_match_schema(schema_name: str, data_path: Path) -> None:
    """The fire rules (core) and fire effects (Godot) files match their schemas."""
    schema = json.loads((DATA / "schemas" / schema_name).read_text(encoding="utf-8"))
    document = json.loads((DATA / data_path).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
