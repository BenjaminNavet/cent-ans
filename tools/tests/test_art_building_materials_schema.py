"""Validates the building materials catalogue against its schema (lot GA5)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _document() -> dict:
    return json.loads(
        (DATA / "art" / "building_materials.json").read_text(encoding="utf-8")
    )


def test_building_materials_match_schema() -> None:
    """``data/art/building_materials.json`` matches ``art_building_materials.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "art_building_materials.schema.json").read_text(
            encoding="utf-8"
        )
    )
    document = _document()
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_building_materials_names_unique() -> None:
    """``textured``/``plain`` names are unique across both lists (shared ``material()`` lookup)."""
    document = _document()
    names = [m["name"] for m in document["textured"]] + [
        m["name"] for m in document["plain"]
    ]
    assert len(names) == len(set(names))


def test_atlas_layers_are_known_materials() -> None:
    """Every ``atlas_layers`` entry names a ``textured``/``plain`` material."""
    document = _document()
    names = {m["name"] for m in document["textured"]} | {
        m["name"] for m in document["plain"]
    }
    for layer in document["atlas_layers"]:
        assert layer in names, layer


def test_timber_frame_present_and_unwired() -> None:
    """Lot GA5 adds the half-timber (torchis/colombage) material, not yet in the kit's atlas."""
    document = _document()
    by_name = {m["name"]: m for m in document["textured"]}
    assert "TimberFrame" in by_name
    assert by_name["TimberFrame"].get("wired", True) is False
    assert "TimberFrame" not in document["atlas_layers"]
