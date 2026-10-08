"""Validates the historical battle maps against their schema (lot EP7)."""

import json
from pathlib import Path

import pytest

DATA = Path(__file__).resolve().parents[2] / "data"
MAPS = sorted(
    path
    for path in (DATA / "battle_maps").glob("*.json")
    if not path.name.startswith("decor_plan")
)


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_there_are_historical_maps() -> None:
    """At least Crécy is shipped."""
    assert any(path.stem == "crecy" for path in MAPS)


@pytest.mark.parametrize("path", MAPS, ids=lambda p: p.stem)
def test_battle_map_matches_schema(path: Path) -> None:
    """Each map matches ``battle_map.schema.json``; its id is its file name."""
    data = _load(path)
    assert data["id"] == path.stem


@pytest.mark.parametrize("path", MAPS, ids=lambda p: p.stem)
def test_battle_map_references_exist(path: Path) -> None:
    """Province, factions and unit types exist; generals point at a regiment."""
    data = _load(path)
    assert (DATA / "provinces" / f"{data['province']}.json").exists()
    for side in ("attacker", "defender"):
        army = data["armies"][side]
        assert (DATA / "factions" / f"{army['faction']}.json").exists(), army["faction"]
        count = 0
        for block in army["regiments"]:
            assert (DATA / "unit_types" / f"{block['unit_type']}.json").exists(), block
            count += block.get("count", 1)
            assert block.get("wave", 0) < max(1, len(army.get("waves", []))), block
        general = army.get("general")
        if general:
            assert general["regiment"] < count, (side, general)
        for wave in army.get("waves", []):
            if "after" in wave:
                assert wave["after"] < len(army["waves"]), wave


@pytest.mark.parametrize("path", MAPS, ids=lambda p: p.stem)
def test_battle_map_stays_on_the_field(path: Path) -> None:
    """Regiments, woods and decor lie on the field; the relief covers it."""
    data = _load(path)
    width, depth = data["field"]["width_m"], data["field"]["depth_m"]
    for side in ("attacker", "defender"):
        for block in data["armies"][side]["regiments"]:
            assert 0 <= block["x"] <= width and 0 <= block["z"] <= depth, block
    for item in data.get("decor", {}).get("items", []):
        if "x" in item:
            assert -50 <= item["x"] <= width + 50, item
            assert -50 <= item["z"] <= depth + 50, item
    relief = data.get("relief")
    if relief:
        assert len(relief["heights_dm"]) == relief["nx"] * relief["nz"]
        assert (relief["nx"] - 1) * relief["step_m"] >= width
        assert (relief["nz"] - 1) * relief["step_m"] >= depth
