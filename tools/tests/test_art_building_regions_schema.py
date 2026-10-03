"""Validates the regional building style table against its schema (lot TF)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _document() -> dict:
    return json.loads(
        (DATA / "art" / "building_regions.json").read_text(encoding="utf-8")
    )


def _province_regions() -> set[str]:
    return {
        json.loads(path.read_text(encoding="utf-8")).get("region", "")
        for path in (DATA / "provinces").glob("*.json")
    }


def test_building_regions_match_schema() -> None:
    """``data/art/building_regions.json`` matches ``art_building_regions.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "art_building_regions.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_document()))
    assert not errors, [error.message for error in errors]


def test_building_regions_are_province_regions() -> None:
    """Every key of ``regions`` is the ``region`` of at least one province (no typo)."""
    known = _province_regions()
    for region in _document()["regions"]:
        assert region in known, region


def test_north_framed_south_stone() -> None:
    """Timber framing common in the north (Normandy, Flanders, England), rare in the Midi."""
    regions = _document()["regions"]
    for region in ("france_nord", "pays_bas", "angleterre_sud", "angleterre_centre"):
        assert regions[region]["framed"] >= 0.7, region
        assert not regions[region]["southern"], region
    for region in ("aquitaine", "languedoc", "provence_alpes"):
        assert regions[region]["framed"] <= 0.1, region
        assert regions[region]["southern"], region


def test_province_overrides_are_known_provinces() -> None:
    """Every key of ``provinces`` is an existing province file."""
    for province in _document().get("provinces", {}):
        assert (DATA / "provinces" / f"{province}.json").exists(), province
