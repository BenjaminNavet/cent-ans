"""Validates data/rules/starting_armies.json against its schema (lot A6-L3b, ADR 0183)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


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
    return Draft202012Validator(schemas["starting_armies.schema.json"], registry=registry)


def test_starting_armies_match_schema() -> None:
    """The starting armies file exists and matches its schema."""
    validator = _validator()
    Draft202012Validator.check_schema(validator.schema)
    rules = _load(DATA / "rules" / "starting_armies.json")
    errors = sorted(validator.iter_errors(rules), key=lambda e: list(e.path))
    assert not errors, [error.message for error in errors]


def test_factions_and_units_exist() -> None:
    """Every faction and unit type named by the file is real data."""
    rules = _load(DATA / "rules" / "starting_armies.json")
    for faction in rules["factions"]:
        assert (DATA / "factions" / f"{faction}.json").is_file(), faction
    armies = [
        rules["default_army"],
        *rules["factions"].values(),
        *rules["garrisons"].values(),
    ]
    for army in armies:
        for unit in army:
            assert (DATA / "unit_types" / f"{unit}.json").is_file(), unit
