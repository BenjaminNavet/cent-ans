"""Validates the battle ground layers against their schema (lot GA2)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_battle_ground_layers_match_schema() -> None:
    """``data/fx/battle_ground_layers.json`` matches ``fx_battle_ground_layers.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "fx_battle_ground_layers.schema.json").read_text(
            encoding="utf-8"
        )
    )
    document = json.loads(
        (DATA / "fx" / "battle_ground_layers.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_battle_ground_layers_ids_unique() -> None:
    """Layer ``id`` values are unique (used as GDScript role lookup keys)."""
    document = json.loads(
        (DATA / "fx" / "battle_ground_layers.json").read_text(encoding="utf-8")
    )
    ids = [layer["id"] for layer in document["layers"]]
    assert len(ids) == len(set(ids))
