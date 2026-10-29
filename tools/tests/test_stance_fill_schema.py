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


def test_band_defaults_keep_the_wash_off_the_province_core() -> None:
    """VIS-R2: 3D reads the stance by an inner band of 4-8 px; the flat wash only returns far out."""
    tuning = _load("map/stance_fill.json")
    band = tuning["band"]
    assert 4 <= band["width_px"]["mid"] <= band["width_px"]["near"] <= 8
    assert band["width_px"]["near_distance"] < band["width_px"]["mid_distance"]
    assert band["alpha"]["other"] < min(band["alpha"]["self"], band["alpha"]["enemy"], band["alpha"]["friend"])
    assert tuning["zoom"]["flat_from_distance"] >= tuning["zoom"]["far_distance"]
