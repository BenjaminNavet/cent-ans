"""Validates data/rules/crusade.json against its schema (lot JR1, ADR 0165)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rules() -> dict:
    return _load(DATA / "rules" / "crusade.json")


def _validator() -> Draft202012Validator:
    """Builds a validator that resolves `common.schema.json` references locally."""
    schemas = {
        path.name: _load(path) for path in (DATA / "schemas").glob("*.schema.json")
    }
    registry = Registry()
    for name, schema in schemas.items():
        resource = Resource.from_contents(schema)
        registry = registry.with_resource(schema["$id"], resource).with_resource(
            name, resource
        )
    return Draft202012Validator(schemas["crusade_rules.schema.json"], registry=registry)


def test_crusade_rules_match_schema() -> None:
    """The crusade rules file exists and matches its schema."""
    validator = _validator()
    Draft202012Validator.check_schema(validator.schema)
    errors = sorted(validator.iter_errors(_rules()), key=lambda e: list(e.path))
    assert not errors, [error.message for error in errors]


def test_provinces_exist_and_coast_is_holy_land() -> None:
    """The target and the Holy Land are real provinces; the coast is part of it."""
    rules = _rules()
    holy_land = set(rules["holy_land"])
    assert rules["target_province"] in holy_land
    assert set(rules["coastal_holy_land"]) <= holy_land
    for province in holy_land:
        assert (DATA / "provinces" / f"{province}.json").is_file(), province
    for province in rules["coastal_holy_land"]:
        assert _load(DATA / "provinces" / f"{province}.json")["coastal"], province


def test_units_exist_and_are_not_mercenaries() -> None:
    """Contingents and the starting army reuse existing, ordinary unit types."""
    rules = _rules()
    units = [entry["unit"] for entry in rules["passage"]["unit_table"]]
    assert len(units) == len(set(units)), "a unit type listed twice"
    for unit in units + rules["starting_army"]:
        path = DATA / "unit_types" / f"{unit}.json"
        assert path.is_file(), unit
        assert not _load(path).get("mercenary"), unit


def test_base_settlement_is_a_port() -> None:
    """The base is an existing port: the passage needs one."""
    base = _rules()["base_settlement"]
    settlements = [
        settlement
        for path in (DATA / "settlements").glob("prov_*.json")
        for settlement in _load(path)
    ]
    found = [s for s in settlements if s["id"] == base]
    assert found, base
    assert found[0].get("port"), base


def test_scale_is_consistent() -> None:
    """Thresholds are ordered and the spec's scale holds (§ 4.1)."""
    rules = _rules()
    zeal, desertion, fervor = rules["zeal"], rules["desertion"], rules["fervor"]
    assert desertion["threshold"] <= zeal["low_threshold"] < zeal["high_threshold"]
    assert fervor["target_floor"] >= zeal["low_threshold"]
    assert fervor["start"] > zeal["low_threshold"]
    passage = rules["passage"]
    assert passage["delay_turns"] <= passage["cooldown_turns"]
    assert passage["units_base"] <= passage["max_units"]
