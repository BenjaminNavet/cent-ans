"""Validates the regiment formations against their schema (lot RJ-a, ADR 0174)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_battle_map_formations_exist() -> None:
    """Every formation a battle map may name is a key of the formation table."""
    keys = {
        f["key"]
        for f in json.loads(
            (DATA / "rules" / "unit_formations.json").read_text(encoding="utf-8")
        )["formations"]
    }
    schema = json.loads(
        (DATA / "schemas" / "battle_map.schema.json").read_text(encoding="utf-8")
    )
    text = json.dumps(schema)
    start = text.index('"formation": {"enum": [')
    listed = json.loads(
        text[start + len('"formation": {"enum": ') :].split("]")[0] + "]"
    )
    assert set(listed) == keys
