"""Validates data/map/stance_fill.json (lot RJ-d: stance wash on the campaign map, ADR 0175)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_neutrals_stay_lighter_than_stances() -> None:
    """Neutral provinces get a lighter wash than ours and the enemy's; the wash only shows in the political mode by default."""
    tuning = _load("map/stance_fill.json")
    alpha = tuning["alpha"]
    assert alpha["other"] < min(alpha["self"], alpha["enemy"], alpha["friend"])
    assert tuning["zoom"]["near_distance"] < tuning["zoom"]["far_distance"]
    assert "diplomacy" not in tuning["modes"]
