"""Validates data/ui/war_scars.json (lot TB4: visible scars of war and plague on the map)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_battlefield_fades_in_order() -> None:
    """Crows leave first, then the debris, and the mound outlasts both."""
    battlefield = _load("ui/war_scars.json")["battlefield"]
    assert (
        battlefield["crow_turns"] <= battlefield["debris_turns"] <= battlefield["turns"]
    )
    assert battlefield["turns"] >= 2


def test_plague_counts_grow_with_intensity() -> None:
    """A stronger plague never shows fewer pits or marked doors."""
    plague = _load("ui/war_scars.json")["plague"]
    for key in ("pits", "doors"):
        low, high = plague[key]
        assert low <= high


def test_siege_engines_cover_the_rules_and_existing_models() -> None:
    """Every engine kind of the siege rules has a camp entry, and its model file exists."""
    engines = _load("ui/war_scars.json")["siege"]["engines"]
    kinds = {engine["kind"] for engine in _load("rules/siege_engines.json")["engines"]}
    assert kinds <= set(engines)
    for entry in engines.values():
        model = entry.get("model")
        if model:
            assert (ROOT / "game" / model.removeprefix("res://")).exists(), model


def test_core_remembers_battles_as_long_as_the_map_marks_them() -> None:
    """A battlefield mark never outlives its record in the core history."""
    marked_turns = _load("ui/war_scars.json")["battlefield"]["turns"]
    rules = _load("rules/battle_history.json")
    assert marked_turns <= rules["max_age_turns"]
    assert rules["max_records"] >= 1
