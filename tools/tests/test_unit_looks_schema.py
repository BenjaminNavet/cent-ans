"""Validates data/fx/unit_looks.json against unit_looks.schema.json (lot OMR R5)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_unit_looks_name_real_unit_types() -> None:
    """Every unit type with a look exists in data/unit_types."""
    unit_types = {path.stem for path in (DATA / "unit_types").glob("unit_*.json")}
    looks = _load("fx/unit_looks.json")["looks"]
    assert set(looks) <= unit_types, set(looks) - unit_types
