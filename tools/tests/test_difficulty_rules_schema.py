"""Validates data/rules/difficulty.json against its schema (lot DF1, difficulty levels)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"

NEUTRAL = {
    "ai_income_percent": 100,
    "player_income_percent": 100,
    "ai_upkeep_percent": 100,
    "ai_recruit_cost_percent": 100,
    "player_unrest": 0,
    "ai_attitude_to_player": 0,
    "ai_war_ratio_percent": 100,
    "ai_morale_vs_player": 0,
}


def test_difficulty_rules_match_schema() -> None:
    """The four levels exist once each, in order, and ``normal`` is neutral."""
    rules = json.loads((DATA / "rules" / "difficulty.json").read_text(encoding="utf-8"))
    ids = [level["id"] for level in rules["levels"]]
    assert ids == ["easy", "normal", "hard", "very_hard"]
    normal = next(level for level in rules["levels"] if level["id"] == "normal")
    assert {key: normal[key] for key in NEUTRAL} == NEUTRAL
