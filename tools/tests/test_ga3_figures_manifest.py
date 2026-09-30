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


def test_generated_figures_replace_fine_foot_figures():
    """Every entry: known fine foot figure, files present, triangles under the caps."""
    script = _script()
    manifest = json.loads((GA3_DIR / "manifest.json").read_text())
    fine = json.loads(FINE_MANIFEST.read_text())["figures"]
    by_figure = {u["figure"]: name for name, u in script.UNITS.items()}
    figures = manifest["figures"]
    assert {"archer_0", "infantry_0", "archer_2", "infantry_1", "infantry_5"} <= set(
        figures
    )
    for fig, entry in figures.items():
        assert fine[fig]["rig"] == "human", fig
        assert by_figure[fig] == entry["unit"], fig
        assert (GA3_DIR / entry["ga3_albedo"]).exists(), fig
        tris = entry["tris"]
        assert all(t <= cap for t, cap in zip(tris, script.TRI_CAP, strict=True)), fig
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
