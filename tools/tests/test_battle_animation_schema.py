"""Validates data/fx/battle_animation.json (lot NT7: clip blends and role clips)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_cycle_blend_is_short() -> None:
    """The melee cycle blend stays a short cross-fade (0.15-0.25 s, NT7)."""
    blend = _load("fx/battle_animation.json")["cycle_blend_s"]
    assert 0.15 <= blend <= 0.25


def test_role_blend_is_short() -> None:
    """The role clip blend (bearers, musicians, crews) stays a short cross-fade (NT10)."""
    blend = _load("fx/battle_animation.json").get("role_blend_s", 0.0)
    assert 0.15 <= blend <= 0.35
