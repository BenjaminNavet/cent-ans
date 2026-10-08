"""Validates the drag formation width rules against their schema (CB1)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "formation_width.json").read_text(encoding="utf-8")
    )


def test_rank_bounds_are_ordered_and_match_the_plan() -> None:
    """Plan CB1: pikemen keep at least 4 ranks, archers at least 2."""
    classes = _rules()["classes"]
    for name, bounds in classes.items():
        assert bounds["min_ranks"] <= bounds["max_ranks"], name
    assert classes["pikemen"]["min_ranks"] >= 4
    assert classes["ranged"]["min_ranks"] >= 2


def test_default_line_ranks_lie_within_bounds() -> None:
    """The default Line depth (4 foot, 3 shooters, 2 horse, 1 engine) stays valid."""
    classes = _rules()["classes"]
    defaults = {"infantry": 4, "pikemen": 4, "ranged": 3, "cavalry": 2, "siege": 1}
    for name, ranks in defaults.items():
        assert classes[name]["min_ranks"] <= ranks <= classes[name]["max_ranks"], name
