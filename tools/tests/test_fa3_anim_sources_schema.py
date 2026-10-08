"""Validates data/fx/fa3_anim_sources.json against its schema (lot FA3, CC0 animations)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
FINE = ROOT / "game" / "assets" / "models" / "battle_fine"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_fa3_anim_references_resolve() -> None:
    """Every clip part names a declared source, every source a declared skeleton."""
    table = _load(DATA / "fx" / "fa3_anim_sources.json")
    for source in table["sources"].values():
        assert source["skeleton"] in table["skeletons"], source
    names = [clip["clip"] for clip in table["clips"]]
    assert len(names) == len(set(names)), "each game clip is substituted once"
    for clip in table["clips"]:
        for part in clip["parts"]:
            assert part["source"] in table["sources"], (clip["clip"], part)
        if "bow" in clip:
            assert clip["bow"]["draw_from"] < clip["bow"]["release"], clip["clip"]


def test_fa3_anim_clips_exist_in_the_fine_rig() -> None:
    """A substituted clip replaces a clip of the fine `human` rig, with the same loop flag."""
    table = _load(DATA / "fx" / "fa3_anim_sources.json")
    rig = _load(FINE / "manifest.json")["rigs"]["human"]["clips"]
    for clip in table["clips"]:
        assert clip["clip"] in rig, clip["clip"]
        assert rig[clip["clip"]]["loop"] == clip["loop"], clip["clip"]


def test_fa3_anim_bake_matches_the_table() -> None:
    """The baked manifest holds exactly the clips of the table, each with its measures."""
    table = _load(DATA / "fx" / "fa3_anim_sources.json")
    baked = _load(FINE / "fa3_anim" / "manifest.json")
    names = {clip["clip"] for clip in table["clips"]}
    assert set(baked["clips"]) == names
    assert set(baked["clip_sources"]) == names
    defaults = {clip["clip"]: clip["default"] for clip in table["clips"]}
    for name, clip in baked["clips"].items():
        assert clip["default"] is defaults[name], name
    for name, record in baked["clip_sources"].items():
        assert record["license"] == "CC0-1.0", name
        assert "foot_slide_cm" in record["quality"], name
        assert "loop_gap_cm" in record["quality"], name


def test_fa3_anim_defaults_are_the_judged_clips() -> None:
    """The clips played without an option are flagged in the table, never in code."""
    table = _load(DATA / "fx" / "fa3_anim_sources.json")
    defaults = {clip["clip"] for clip in table["clips"] if clip["default"]}
    assert defaults, "at least one default clip"
    assert defaults < {clip["clip"] for clip in table["clips"]}, (
        "some clips stay trials"
    )
    script = (ROOT / "game" / "scripts" / "battle" / "battle_skinned.gd").read_text(
        encoding="utf-8"
    )
    fa3 = script[script.index("static func fa_anim_mode") :]
    fa3 = fa3[: fa3.index("\n\n\n")]
    for name in table["clips"]:
        assert f'"{name["clip"]}"' not in fa3, name["clip"]
