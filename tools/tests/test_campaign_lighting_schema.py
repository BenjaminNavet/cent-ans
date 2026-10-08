"""Validates data/fx/campaign_lighting.json (lot RV-B: campaign game lighting)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_campaign_lighting_is_coherent() -> None:
    """Shade is cooler than the sun side, haze is lighter seen from above (ADR 0142)."""
    lighting = _load("fx/campaign_lighting.json")
    ambient = lighting["ambient"]["color"]
    assert ambient[2] > ambient[0], "ambient light should be a cool sky blue"
    low, high = lighting["elevation_bounds_deg"]
    assert low < high
    aerial = lighting["aerial"]
    assert aerial["low_pitch_deg"] < aerial["top_pitch_deg"]
    for key in ("density", "aerial_perspective"):
        assert aerial[key][1] <= aerial[key][0], key
    assert aerial["begin_factor"][1] >= aerial["begin_factor"][0]
