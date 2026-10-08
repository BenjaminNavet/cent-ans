"""Validates data/rules/battle_outcome.json against battle_outcome_rules.schema.json (lot CV3-1)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_victories_rank_above_defeats() -> None:
    """Heroic and decisive victories pay more prestige than any defeat."""
    classes = _load(DATA / "rules" / "battle_outcome.json")["classes"]
    worst_victory = min(classes[k]["prestige"] for k in ("heroic", "decisive"))
    best_defeat = max(
        classes[k]["prestige"] for k in ("honourable_defeat", "disaster", "defeat")
    )
    assert worst_victory > best_defeat
