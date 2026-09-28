"""Validates data/rules/mercenaries.json against its schema (lot TW2-T3, ADR 0103)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rules() -> dict:
    return _load(DATA / "rules" / "mercenaries.json")


def _regions() -> set[str]:
    return {_load(path)["region"] for path in (DATA / "provinces").glob("*.json")}


def test_mercenary_rules_match_schema() -> None:
    """The mercenary rules file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "mercenary_rules.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path)
    assert not errors, [error.message for error in errors]


def test_upkeep_and_price_are_a_premium() -> None:
    """Companies cost more than levies: upkeep ×1.5-2, a dear hiring price (spec § T3)."""
    rules = _rules()
    assert 150 <= rules["upkeep_percent"] <= 200
    assert rules["hire_cost_percent"] > 100


def test_every_region_is_named() -> None:
    """Every province region has a French name, and no name is stale."""
    assert set(_rules()["region_names"]) == _regions()


def test_bands_name_mercenary_units_and_known_regions() -> None:
    """Bands hire units marked `mercenary`, in existing regions, within the unit's period."""
    regions = _regions()
    ids = set()
    places = set()
    for band in _rules()["bands"]:
        assert band["id"] not in ids, band["id"]
        ids.add(band["id"])
        assert band["from"] <= band["until"], band["id"]
        unit = _load(DATA / "unit_types" / f"{band['unit']}.json")
        assert unit.get("mercenary") is True, band["unit"]
        assert band["from"] >= unit.get("available_from", 0), band["id"]
        assert band["until"] <= unit.get("available_until", 9999), band["id"]
        for region in band["regions"]:
            assert region in regions, (band["id"], region)
            # One reserve per region and unit type.
            assert (region, band["unit"]) not in places, (band["id"], region)
            places.add((region, band["unit"]))


def test_every_mercenary_unit_can_be_hired_somewhere() -> None:
    """A unit marked `mercenary` is only raised from a company reserve: one must offer it."""
    hired = {band["unit"] for band in _rules()["bands"]}
    for path in (DATA / "unit_types").glob("*.json"):
        if _load(path).get("mercenary"):
            assert path.stem in hired, path.stem
