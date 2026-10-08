"""Validates data/ui/campaign_map.json (lot CV3-5: TW-scale lord on the campaign map)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_lord_is_larger_than_his_escort() -> None:
    """The general stands out from the escort (TW-scale lord)."""
    assert _load("ui/campaign_map.json")["map"]["army_figure_scale"] > 1.0
