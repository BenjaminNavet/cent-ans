"""GA3-L3: manifest of the generated battle figures (``game/assets/models/battle_ga3``)."""

import importlib.util
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
GA3_DIR = ROOT / "game/assets/models/battle_ga3"
FINE_MANIFEST = ROOT / "game/assets/models/battle_fine/manifest.json"
SCRIPT = ROOT / "tools/blender_scripts/ga3_figures.py"


def _script():
    spec = importlib.util.spec_from_file_location("ga3_figures", SCRIPT)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def test_generated_figures_replace_fine_figures():
    """Every entry: known fine figure, files present, triangles under the caps.

    Foot figures: the foot soldiers' caps; the knight (L3c): the rider cap plus the fine
    horse of the exported fine figure.
    """
    script = _script()
    manifest = json.loads((GA3_DIR / "manifest.json").read_text())
    fine = json.loads(FINE_MANIFEST.read_text())["figures"]
    by_figure = {u["figure"]: name for name, u in script.UNITS.items()}
    figures = manifest["figures"]
    assert {
        "archer_0",
        "infantry_0",
        "archer_2",
        "infantry_1",
        "infantry_5",
        "cavalry_0",
    } <= set(figures)
    for fig, entry in figures.items():
        mounted = entry.get("fine_horse", False)
        assert fine[fig]["rig"] == ("cavalry" if mounted else "human"), fig
        assert by_figure[fig] == entry["unit"], fig
        assert (GA3_DIR / entry["ga3_albedo"]).exists(), fig
        tris = entry["tris"]
        caps = (
            [r + f for r, f in zip(script.RIDER_CAP, fine[fig]["tris"], strict=True)]
            if mounted
            else script.TRI_CAP
        )
        assert all(t <= cap for t, cap in zip(tris, caps, strict=True)), fig
        assert tris[0] > tris[1] > tris[2], fig
        for lod in entry["lods"]:
            assert (GA3_DIR / lod).exists(), lod
        assert entry["variants"] == 1


def test_units_are_data():
    """Each unit declares its figure, held items, steel height and render clips."""
    for name, unit in _script().UNITS.items():
        assert unit["equipment"], name
        assert set(unit["clips"]) == {"idle", "walk", "attack"}, name
        assert 0.0 <= unit["metal_z"] <= 2.0, name


def test_knight_horse_is_the_exported_fine_horse():
    """L3c: the horse triangles of the knight are those of the fine ``cavalry_0``.

    The horse keeps the FG3 atlas UVs of the fine figure (``atlas_layer``): rebuilding the
    fine figure requires rebuilding the knight (``ga3_figures.py -- knight``).
    """
    script = _script()
    bones = json.loads(FINE_MANIFEST.read_text())["rigs"]["cavalry"]["bones"]
    fine_dir = FINE_MANIFEST.parent
    for level in range(3):
        name = f"cavalry_0_lod{level}.mesh.bin"
        counts = []
        for path in (fine_dir / name, GA3_DIR / name):
            cam = script.read_cam(str(path))
            horse = [
                all(
                    not bones[round(b)].startswith("R:")
                    for b, w in zip(bb, ww, strict=True)
                    if w > 0
                )
                for bb, ww in zip(cam["bone"], cam["weight"], strict=True)
            ]
            idx = cam["idx"]
            counts.append(
                sum(
                    1
                    for t in range(0, len(idx), 3)
                    if all(horse[i] for i in idx[t : t + 3])
                )
            )
            if path.parent == GA3_DIR and level < 2:
                assert cam["atlas"] is not None, name
        assert counts[0] == counts[1] > 0, (name, counts)
