"""Validates the map flames and wind parameters against their schema (lot AS5)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_map_fire_wind_matches_schema() -> None:
    """data/fx/map_fire_wind.json matches fx_map_fire_wind.schema.json."""
    schema = json.loads(
        (DATA / "schemas" / "fx_map_fire_wind.schema.json").read_text(encoding="utf-8")
    )
    document = json.loads((DATA / "fx" / "map_fire_wind.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path)
    assert not errors, [error.message for error in errors]
