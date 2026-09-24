"""Validates data/buildings/*.json and their required resources (P1, tin)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

DATA = Path(__file__).resolve().parents[2] / "data"


def _validator() -> Draft202012Validator:
    """Builds a validator that resolves `common.schema.json` references locally."""
    schemas = {
        path.name: json.loads(path.read_text(encoding="utf-8"))
        for path in (DATA / "schemas").glob("*.schema.json")
    }
    registry = Registry()
    for name, schema in schemas.items():
        resource = Resource.from_contents(schema)
        registry = registry.with_resource(schema["$id"], resource).with_resource(
            name, resource
        )
    return Draft202012Validator(schemas["building.schema.json"], registry=registry)


def _load(folder: str) -> dict[str, dict]:
    """All JSON entities of a data folder, by id."""
    return {
        entity["id"]: entity
        for entity in (
            json.loads(path.read_text(encoding="utf-8"))
            for path in sorted((DATA / folder).glob("*.json"))
        )
    }


def test_every_building_matches_the_schema() -> None:
    """Each building is valid and its file name equals its id."""
    validator = _validator()
    for path in sorted((DATA / "buildings").glob("*.json")):
        building = json.loads(path.read_text(encoding="utf-8"))
        errors = [error.message for error in validator.iter_errors(building)]
        assert not errors, f"{path.name}: {errors}"
        assert path.stem == building["id"]


def test_required_resources_exist_somewhere_on_the_map() -> None:
    """A required resource is a real resource found in at least one province."""
    resources = _load("resources")
    on_map = {
        resource
        for province in _load("provinces").values()
        for resource in province.get("resources", [])
    }
    for building in _load("buildings").values():
        required = building.get("required_resource")
        if required is None:
            continue
        assert required in resources, f"{building['id']}: unknown {required}"
        assert required in on_map, f"{building['id']}: {required} is on no province"


def test_tin_feeds_the_blowing_house() -> None:
    """Tin (Cornwall, Devon) is used by the tin blowing house, which needs a river."""
    buildings = _load("buildings")
    users = sorted(
        id_ for id_, b in buildings.items() if b.get("required_resource") == "res_tin"
    )
    assert users == ["bld_tin_blowing_house"]
    provinces = _load("provinces")
    tin_provinces = sorted(
        id_ for id_, p in provinces.items() if "res_tin" in p.get("resources", [])
    )
    assert tin_provinces == ["prov_cornwall", "prov_devon"]
    assert buildings["bld_tin_blowing_house"]["requires_river"]
    for province in tin_provinces:
        assert provinces[province]["rivers"], f"{province} needs a river"
