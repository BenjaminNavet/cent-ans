"""Validates the battle path preview and hover cursor rules against their schema (CB-M2)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_hover.json").read_text(encoding="utf-8")
    )


def test_preview_throttle_matches_the_spec() -> None:
    """Spec CB-M: recompute beyond 5 m, at most 10 per second, one path beyond 6 units."""
    preview = _rules()["preview"]
    assert preview["recompute_distance_m"] == 5.0
    assert preview["max_recomputes_per_s"] == 10.0
    assert preview["max_individual_paths"] == 6


def test_fire_arc_is_a_forward_sector() -> None:
    """CB-M4: the drawn fire sector is centred on the facing and narrower than a half turn."""
    half_angle = _rules()["range_arc"]["fire_half_angle_deg"]
    assert 0.0 < half_angle < 90.0
