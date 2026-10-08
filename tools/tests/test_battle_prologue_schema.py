"""Validates data/tutorial/battle_prologue.json against its schema (lot NT4)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_battle_prologue_references_exist() -> None:
    """Factions and unit types of the prologue armies are real data entries."""
    battle = _load("tutorial/battle_prologue.json")["battle"]
    for side in ("attacker", "defender"):
        army = battle[side]
        assert (DATA / "factions" / f"{army['faction']}.json").is_file(), army
        for unit in army["units"]:
            assert (DATA / "unit_types" / f"{unit}.json").is_file(), unit


def test_battle_prologue_steps_cover_the_lesson() -> None:
    """Steps are unique, start and end manually and teach every required skill."""
    steps = _load("tutorial/battle_prologue.json")["steps"]
    ids = [step["id"] for step in steps]
    assert len(ids) == len(set(ids))
    assert steps[0]["condition"]["type"] == "manual"
    assert steps[-1]["condition"]["type"] == "manual"
    kinds = {step["condition"]["type"] for step in steps}
    assert {
        "camera_moved",
        "selection",
        "moved",
        "formation_changed",
        "unit_state",
        "paused",
        "victory",
    } <= kinds
    assert any(step.get("enemy_ai") for step in steps), "the enemy never wakes up"
