"""Tests for the Blender model pipeline (parsing always, export if Blender exists)."""

from pathlib import Path

import pytest

from cent_ans_tools import blender


def test_parse_model_counts() -> None:
    """Only well-formed ``MODEL`` lines are kept."""
    output = "Blender 5.2\nMODEL castle 618\nMODEL ship 116\nnoise MODEL\nOK\n"
    assert blender.parse_model_counts(output) == {"castle": 618, "ship": 116}


@pytest.mark.skipif(not blender.BLENDER_BIN.exists(), reason="Blender absent")
def test_build_one_model(tmp_path: Path) -> None:
    """The ship exports as a non-empty GLB under the triangle budget."""
    counts = blender.build_models(tmp_path, "ship")
    assert 0 < counts["ship"] < blender.MAX_TRIANGLES
    assert (tmp_path / "ship.glb").stat().st_size > 1000


def test_models_script_lists_v3_models() -> None:
    """The Blender script declares every model the game loads (lot V3)."""
    source = (blender.SCRIPTS_DIR / "models.py").read_text(encoding="utf-8")
    for name in (
        "castle",
        "city_cathedral",
        "town",
        "village",
        "cathedral",
        "army",
        "army_foot",
        "army_archer",
        "siege_camp",
        "ship",
    ):
        assert f'"{name}": build_' in source
