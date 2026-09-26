"""Validates data/map/faction_borders.json (lot FR1: glowing faction borders)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_faction_borders_matches_schema() -> None:
    """The tuning file matches its schema."""
    schema = _load("schemas/faction_borders.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_load("map/faction_borders.json")))
    assert not errors, [error.message for error in errors]


def test_faction_borders_consistency() -> None:
    """Fades are ordered, realm borders are bolder than province borders, political mode shows them."""
    tuning = _load("map/faction_borders.json")
    assert tuning["zoom"]["fade_out_m"] < tuning["zoom"]["fade_in_m"]
    assert tuning["realm"]["core_px"] > tuning["province"]["core_px"]
    assert tuning["realm"]["core_alpha"] > tuning["province"]["core_alpha"]
    assert tuning["modes"]["political"]["alpha"] == 1.0
    assert not tuning["modes"]["political"]["neutral"]
