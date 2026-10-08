"""Validates the battle ground layers against their schema (lot GA2)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_battle_ground_layers_ids_unique() -> None:
    """Layer ``id`` values are unique (used as GDScript role lookup keys)."""
    document = json.loads(
        (DATA / "fx" / "battle_ground_layers.json").read_text(encoding="utf-8")
    )
    ids = [layer["id"] for layer in document["layers"]]
    assert len(ids) == len(set(ids))
