"""Validates the standards files of lot EP5 (rules and rendering) against their schemas."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_standard_kinds_and_unit_types_exist() -> None:
    """Every kind named is defined; every unit type named exists."""
    fx = _load("fx/battle_standards.json")
    kinds = set(fx["kinds"])
    unit_types = {path.stem for path in (DATA / "unit_types").glob("unit_*.json")}
    named = [fx["default_kind"], fx["second_bearer_kind"], fx["general"]["kind"]]
    named += list(fx["by_unit_type"].values())
    for key in ("by_faction", "sovereign_by_faction", "no_quarter_by_faction"):
        named += list(fx["general"].get(key, {}).values())
    assert set(named) <= kinds, set(named) - kinds
    assert set(fx["by_unit_type"]) <= unit_types
    assert set(fx["musicians"]["by_unit_type"]) <= unit_types


def test_standard_figures_and_clips_are_baked() -> None:
    """The bearer and musician figures and their clips exist in the manifest."""
    manifest = json.loads(
        (ROOT / "game/assets/models/battle_skinned/manifest.json").read_text(
            encoding="utf-8"
        )
    )
    figures = manifest["figures"]
    for name in ("standard_0", "standard_1", "musician_0", "musician_1"):
        assert name in figures, name
    for name in ("standard_0", "standard_1"):
        assert len(figures[name]["pole_top"]) == 3
        assert len(figures[name]["pole_axis"]) == 3
    human = set(manifest["rigs"]["human"]["clips"])
    cavalry = set(manifest["rigs"]["cavalry"]["clips"])
    assert {"std_idle", "std_walk", "std_run", "std_wave", "std_death"} <= human
    assert {"drum_idle", "drum_march", "drum_beat"} <= human
    assert {"horn_idle", "horn_walk", "horn_blow"} <= human
    assert {
        "c_std_idle",
        "c_std_walk",
        "c_std_gallop",
        "c_std_wave",
        "c_std_death",
    } <= cavalry
    assert len(human) <= 48 and len(cavalry) <= 48
