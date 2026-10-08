"""Validates data/ui/map_legend.json (lot UX1: campaign map legend)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_map_legend_references_exist() -> None:
    """Example factions exist and section ids are unique."""
    legend = _load("ui/map_legend.json")
    for faction in legend["factions"]:
        assert (DATA / "factions" / f"{faction}.json").is_file(), faction
    ids = [section["id"] for section in legend["sections"]]
    assert len(ids) == len(set(ids)), ids


def test_every_map_mode_explains_the_land_colours() -> None:
    """Each titled map mode has its own section, and sections only use titled modes."""
    legend = _load("ui/map_legend.json")
    modes = set(legend["mode_titles"])
    for mode in modes:
        assert any(
            mode in section.get("modes", []) for section in legend["sections"]
        ), mode
    for section in legend["sections"]:
        assert set(section.get("modes", [])) <= modes, section["id"]
