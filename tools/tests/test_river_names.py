"""Validates data/map/river_names.json against its schema and source names (lot RC3a)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_river_names_keys_exist_and_values_differ() -> None:
    """Every key is a known source name and no value equals its key."""
    names = _load(DATA / "map" / "river_names.json")["names"]
    rivers = _load(DATA / "map" / "rivers_render.json")["rivers"]
    crossings = _load(DATA / "map" / "crossings_px.json")["crossings"]
    known = {r["name"] for r in rivers if r.get("name")}
    known |= {c["river"] for c in crossings if c.get("river")}
    unknown = sorted(set(names) - known)
    assert not unknown, unknown
    same = sorted(k for k, v in names.items() if k == v)
    assert not same, same
