"""Validates data/fx/atmosphere.json (lot V3, A1-05: HDRI skies and colour grading)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_atmosphere_matches_schema() -> None:
    """The atmosphere file matches its schema."""
    schema = _load("schemas/fx_atmosphere.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_load("fx/atmosphere.json")))
    assert not errors, [error.message for error in errors]


def test_atmosphere_references_exist() -> None:
    """Every sky referenced by a weather or season exists, and every panorama file is present."""
    atmosphere = _load("fx/atmosphere.json")
    skies = atmosphere["skies"]
    for sky in skies.values():
        assert (ROOT / "game" / sky["path"].removeprefix("res://")).is_file(), sky["path"]
    for weather, look in atmosphere["battle"].items():
        for season, sky_id in look["sky"].items():
            assert sky_id in skies, (weather, season, sky_id)
    for season, look in atmosphere["campaign"]["seasons"].items():
        assert look["sky"] in skies, (season, look["sky"])
