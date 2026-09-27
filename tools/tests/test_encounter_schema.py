"""Validates data/encounters/*.json and data/rules/encounters.json (lot CV3-3)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

DATA = Path(__file__).resolve().parents[2] / "data"


def _validator(name: str) -> Draft202012Validator:
    """Builds a validator for schema `name` resolving sibling schemas locally."""
    schemas = {
        path.name: json.loads(path.read_text(encoding="utf-8"))
        for path in (DATA / "schemas").glob("*.schema.json")
    }
    registry = Registry()
    for file_name, schema in schemas.items():
        resource = Resource.from_contents(schema)
        registry = registry.with_resource(schema["$id"], resource).with_resource(
            file_name, resource
        )
    Draft202012Validator.check_schema(schemas[name])
    return Draft202012Validator(schemas[name], registry=registry)


def test_encounter_rules_match_schema() -> None:
    """The encounter rules file exists and matches its schema."""
    validator = _validator("encounter_rules.schema.json")
    rules = json.loads((DATA / "rules" / "encounters.json").read_text(encoding="utf-8"))
    errors = [error.message for error in validator.iter_errors(rules)]
    assert not errors, errors


def test_every_encounter_matches_the_schema() -> None:
    """About twelve encounters, each valid, sourced, file name equal to id."""
    validator = _validator("encounter.schema.json")
    files = sorted((DATA / "encounters").glob("enc_*.json"))
    assert len(files) >= 12
    for path in files:
        encounter = json.loads(path.read_text(encoding="utf-8"))
        errors = [error.message for error in validator.iter_errors(encounter)]
        assert not errors, f"{path.name}: {errors}"
        assert path.stem == encounter["id"]
        assert encounter["sources"], f"{path.name}: sources vides"
        defaults = [o for o in encounter["options"] if o.get("default")]
        assert len(defaults) <= 1, f"{path.name}: plusieurs options par défaut"
        default = defaults[0] if defaults else encounter["options"][0]
        assert not default.get("conditions"), (
            f"{path.name}: l'option par défaut ne doit pas avoir de condition"
        )
        low, high = encounter["spawn"]["lifetime"]
        assert low <= high, f"{path.name}: lifetime inversée"
