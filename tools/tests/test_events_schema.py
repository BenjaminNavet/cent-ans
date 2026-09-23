"""Validates data/events/*.json against data/schemas/event.schema.json (M10)."""

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
    return Draft202012Validator(schemas["event.schema.json"], registry=registry)


def test_every_event_matches_the_schema() -> None:
    """At least 40 events, each valid, file name equal to id."""
    validator = _validator()
    files = sorted((DATA / "events").glob("*.json"))
    assert len(files) >= 40
    kinds = {"historical": 0, "random": 0, "chained": 0}
    for path in files:
        event = json.loads(path.read_text(encoding="utf-8"))
        errors = [error.message for error in validator.iter_errors(event)]
        assert not errors, f"{path.name}: {errors}"
        assert path.stem == event["id"]
        kinds[event["kind"]] += 1
    assert kinds["historical"] >= 20
    assert kinds["random"] >= 20
