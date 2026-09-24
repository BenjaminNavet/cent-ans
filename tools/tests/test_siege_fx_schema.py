"""Validates data/fx/siege_fx.json against siege_fx.schema.json (lot S1, wall collapse)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_siege_fx_match_schema() -> None:
    """The siege visual effects file exists and matches its schema."""
    schema = _load("schemas/siege_fx.schema.json")
    settings = _load("fx/siege_fx.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(settings), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_siege_fx_ranges_are_ordered() -> None:
    """Every [min, max] pair and count bound is ordered."""
    settings = _load("fx/siege_fx.json")
    assert settings["wall"]["blocks_min"] <= settings["wall"]["blocks_max"]
    assert settings["parapet"]["stones_min"] <= settings["parapet"]["stones_max"]
    for section in ("wall", "gate", "parapet", "dust"):
        for value in settings[section].values():
            if isinstance(value, list) and len(value) == 2:
                assert value[0] <= value[1], (section, value)
