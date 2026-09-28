"""Validates data/battle_orders/*.json against battle_order.schema.json (F10b)."""

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
    return Draft202012Validator(schemas["battle_order.schema.json"], registry=registry)


def test_every_battle_order_matches_the_schema() -> None:
    """The four leader's orders (the pavise became an ability, CB4), each valid."""
    validator = _validator()
    files = sorted((DATA / "battle_orders").glob("*.json"))
    kinds = set()
    for path in files:
        order = json.loads(path.read_text(encoding="utf-8"))
        errors = [error.message for error in validator.iter_errors(order)]
        assert not errors, f"{path.name}: {errors}"
        assert path.stem == order["id"]
        kinds.add(order["kind"])
    assert kinds == {"war_cry", "no_quarter", "dismount", "rally"}


def test_faction_labels_name_existing_factions() -> None:
    """Every faction named in labels or AI rivals exists in data/factions."""
    factions = {path.stem for path in (DATA / "factions").glob("*.json")}
    for path in (DATA / "battle_orders").glob("*.json"):
        order = json.loads(path.read_text(encoding="utf-8"))
        named = set(order.get("labels_by_faction", {}))
        for pair in order.get("ai", {}).get("rivals", []):
            named.update(pair)
        assert named <= factions, f"{path.name}: {named - factions}"
