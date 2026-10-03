"""Validates the battle ground noise data against its schema (lot A6-L14)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_battle_ground_matches_schema() -> None:
    """``data/fx/battle_ground.json`` matches ``fx_battle_ground.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "fx_battle_ground.schema.json").read_text(encoding="utf-8")
    )
    document = json.loads(
        (DATA / "fx" / "battle_ground.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors]
