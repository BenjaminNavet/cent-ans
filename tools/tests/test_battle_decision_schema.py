"""Validates the battle decision rules against their schema (lot EP9)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_an_army_without_its_general_breaks_sooner() -> None:
    """Losing the general can only raise the break threshold."""
    rules = _load(DATA / "rules" / "battle_decision.json")
    assert rules["break_share_without_general"] >= rules["break_share"]
