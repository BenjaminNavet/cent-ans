"""Validates the naval data (lot NV1, ADR 0028) against its schemas."""

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"
NAVAL = DATA / "naval"


def _schema(name: str) -> Draft202012Validator:
    schema = json.loads((DATA / "schemas" / name).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


def _errors(validator: Draft202012Validator, document: dict) -> list[str]:
    return [error.message for error in validator.iter_errors(document)]


@pytest.mark.parametrize("path", sorted((NAVAL / "ships").glob("*.json")), ids=lambda p: p.stem)
def test_ship_classes_match_schema(path: Path) -> None:
    """Every ship class matches ship_class.schema.json and is named after its id."""
    ship = json.loads(path.read_text(encoding="utf-8"))
    assert not _errors(_schema("ship_class.schema.json"), ship)
    assert ship["id"] == path.stem


def test_naval_rules_match_schema() -> None:
    """data/naval/rules.json matches naval_rules.schema.json."""
    rules = json.loads((NAVAL / "rules.json").read_text(encoding="utf-8"))
    assert not _errors(_schema("naval_rules.schema.json"), rules)


def test_fleets_match_schema_and_reference_known_ships() -> None:
    """Fleets use known factions and ship classes."""
    fleets = json.loads((NAVAL / "fleets.json").read_text(encoding="utf-8"))
    assert not _errors(_schema("naval_fleets.schema.json"), fleets)
    ships = {p.stem for p in (NAVAL / "ships").glob("*.json")}
    factions = {p.stem for p in (DATA / "factions").glob("*.json")}
    for fleet in fleets["fleets"]:
        assert fleet["faction"] in factions
        assert set(fleet["ships"]) <= ships


@pytest.mark.parametrize(
    "path", sorted((NAVAL / "scenarios").glob("*.json")), ids=lambda p: p.stem
)
def test_scenarios_match_schema(path: Path) -> None:
    """Scenarios match their schema and reference known ships and units."""
    scenario = json.loads(path.read_text(encoding="utf-8"))
    assert not _errors(_schema("naval_scenario.schema.json"), scenario)
    assert scenario["id"] == path.stem
    ships = {p.stem for p in (NAVAL / "ships").glob("*.json")}
    units = {p.stem for p in (DATA / "unit_types").glob("*.json")}
    for side in ("attacker", "defender"):
        fleet = scenario[side]
        assert {u["unit_type"] for u in fleet["units"]} <= units
        for ship in fleet["ships"]:
            assert ship["class"] in ships
            assert all(c["unit"] < len(fleet["units"]) for c in ship["crew"])
