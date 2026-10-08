"""Validates data/rules/starting_armies.json against its schema (lot A6-L3b, ADR 0183)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_factions_and_units_exist() -> None:
    """Every faction and unit type named by the file is real data."""
    rules = _load(DATA / "rules" / "starting_armies.json")
    for faction in rules["factions"]:
        assert (DATA / "factions" / f"{faction}.json").is_file(), faction
    armies = [
        rules["default_army"],
        *rules["factions"].values(),
        *rules["garrisons"].values(),
    ]
    for army in armies:
        for unit in army:
            assert (DATA / "unit_types" / f"{unit}.json").is_file(), unit
