"""Validates data/battle_abilities/*.json against battle_ability.schema.json (CB4)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

DATA = Path(__file__).resolve().parents[2] / "data"


def _validator() -> Draft202012Validator:
    """Builds a validator that resolves sibling schema references locally."""
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
    return Draft202012Validator(
        schemas["battle_ability.schema.json"], registry=registry
    )


def _abilities() -> list[tuple[Path, dict]]:
    """Every ability file with its parsed content."""
    return [
        (path, json.loads(path.read_text(encoding="utf-8")))
        for path in sorted((DATA / "battle_abilities").glob("*.json"))
    ]


def test_every_battle_ability_matches_the_schema() -> None:
    """The five abilities kept by the historian, each valid, file name equal to id."""
    validator = _validator()
    kinds = set()
    for path, ability in _abilities():
        errors = [error.message for error in validator.iter_errors(ability)]
        assert not errors, f"{path.name}: {errors}"
        assert path.stem == ability["id"]
        kinds.add(ability["kind"])
    assert kinds == {
        "aimed_shot",
        "pavise",
        "banner_rally",
        "close_ranks",
        "planted_pikes",
    }


def test_eligible_unit_types_exist() -> None:
    """Every unit type named by an ability exists in data/unit_types."""
    types = {path.stem for path in (DATA / "unit_types").glob("*.json")}
    for path, ability in _abilities():
        named = set(ability["eligible"].get("unit_types", []))
        assert named <= types, f"{path.name}: {named - types}"


def test_pavise_left_the_leader_orders() -> None:
    """The pavise is now a crossbowmen's ability, no longer a leader's order."""
    kinds = {
        json.loads(path.read_text(encoding="utf-8"))["kind"]
        for path in (DATA / "battle_orders").glob("*.json")
    }
    assert "pavise" not in kinds
    pavise = json.loads(
        (DATA / "battle_abilities" / "ability_pavise.json").read_text(encoding="utf-8")
    )
    assert pavise["effects"]["missile_taken_factor"] == 0.35
    assert pavise["effects"]["frontal_only"] is True
    assert pavise["setup_time"] > 0
