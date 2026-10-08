"""Validates data/map/stance_cues.json (lot EN: enemy / friend / neutral cues on the campaign map)."""

import colorsys
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def _saturation(html_color: str) -> float:
    red, green, blue = (int(html_color[i : i + 2], 16) / 255.0 for i in (1, 3, 5))
    return colorsys.rgb_to_hsv(red, green, blue)[1]


def test_enemy_red_stands_out_from_muted_neutrals() -> None:
    """War maps to the enemy category and its border is far more saturated than any neutral one."""
    tuning = _load("map/stance_cues.json")
    assert tuning["categories"]["war"] == "enemy"
    assert tuning["categories"]["self"] == "self"
    border = tuning["border"]
    assert border["other"]["value_min"] < border["other"]["value_max"]
    assert _saturation(border["enemy"]) >= border["other"]["saturation_max"] + 0.3
