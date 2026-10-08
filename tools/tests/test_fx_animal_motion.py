"""Validates data/fx/animal_motion.json (lot AS1) against its schema and checks its coherence."""

import json
from pathlib import Path

from conftest import assert_matches_schema

DATA = Path(__file__).resolve().parents[2] / "data"


def _settings() -> dict:
    return json.loads((DATA / "fx" / "animal_motion.json").read_text(encoding="utf-8"))


def test_animal_motion_matches_schema() -> None:
    """The animal motion settings match their schema."""
    assert_matches_schema("fx/animal_motion.json", "fx_animal_motion.schema.json")


def test_cart_bases_exist() -> None:
    """Every `base` points to another model of the table."""
    models = _settings()["campaign"]["models"]
    for name, entry in models.items():
        base = entry.get("base")
        assert base is None or (base in models and base != name), name
