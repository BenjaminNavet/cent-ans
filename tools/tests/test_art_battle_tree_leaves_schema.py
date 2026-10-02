"""Validates the FA1 battle tree leaf catalogue against its schema."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
TEXTURES = ROOT / "game/assets/textures/battle"


def _document() -> dict:
    return json.loads(
        (DATA / "art" / "battle_tree_leaves.json").read_text(encoding="utf-8")
    )


def test_battle_tree_leaves_match_schema() -> None:
    """``data/art/battle_tree_leaves.json`` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "art_battle_tree_leaves.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_document()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_battle_tree_leaf_textures_exist() -> None:
    """Every catalogued species has its built spray texture, and so do dead leaves."""
    for species in _document()["species"]:
        assert (TEXTURES / f"leaf_spray_{species}.png").is_file(), species
    assert (TEXTURES / "dead_leaves_oak.png").is_file()
