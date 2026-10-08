"""Validates data/speeches/battle_speeches.json (lot BV3, general's speech before battle)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"
BATTLE_TERRAINS = {
    "plains",
    "heath",
    "bocage",
    "forest",
    "hills",
    "mountains",
    "marsh",
    "steppe",
    "desert",
}


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_speech_keys_are_known() -> None:
    """Faction keys are real factions and terrain keys are battle terrains."""
    speeches = _load("speeches/battle_speeches.json")
    factions = {path.stem for path in (DATA / "factions").glob("fac_*.json")}
    for section in ("openings", "closings"):
        for key in speeches[section]:
            assert key == "default" or key in factions, (section, key)
    for key in speeches["terrain"]:
        assert key == "default" or key in BATTLE_TERRAINS, key


def test_general_lines_name_the_general() -> None:
    """Every general line names the general through the {general} placeholder."""
    speeches = _load("speeches/battle_speeches.json")
    for line in speeches["general_lines"]:
        assert "{general}" in line, line
