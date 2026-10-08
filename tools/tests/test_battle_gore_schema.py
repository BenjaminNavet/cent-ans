"""Validates data/fx/battle_gore.json against battle_gore.schema.json (lot BV2, deaths and gore)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_battle_gore_ranges_are_ordered() -> None:
    """Every [min, max] pair is ordered and every unit type has a weapon."""
    settings = _load("fx/battle_gore.json")
    ranges = [death["impulse"] for death in settings["deaths"].values()]
    ranges += [settings["knockdown"]["impulse"], settings["sprays"]["speed"]]
    ranges.append(settings["severed"]["speed"])
    for low, high in ranges:
        assert low <= high
    unit_types = {path.stem for path in (DATA / "unit_types").glob("unit_*.json")}
    assert unit_types <= set(settings["weapons"]), unit_types - set(settings["weapons"])
