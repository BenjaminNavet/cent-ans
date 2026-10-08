"""Validates the battle scale rules against their schema (lot EP1)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_battle_scale_tiers_are_ordered() -> None:
    """Tiers grow with the head count; only the last one is unbounded."""
    tiers = _load(DATA / "rules" / "battle_scale.json")["tiers"]
    bounds = [tier.get("max_soldiers") for tier in tiers]
    assert all(bound is not None for bound in bounds[:-1])
    assert bounds[-1] is None
    assert bounds[:-1] == sorted(bounds[:-1])
    for tier in tiers:
        # Both deployment zones fit in the depth of the field.
        assert tier["line_gap_m"] + 2 * (tier["zone_depth_m"] - 50) <= tier["depth_m"]
