"""Validates the campaign terrain texture data against its schema (lot GA4)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"
SHADER_ORDER = ["grass", "farmland", "forest", "rock", "heath", "snow", "sand"]


def _document() -> dict:
    return json.loads(
        (DATA / "fx" / "campaign_terrain_textures.json").read_text(encoding="utf-8")
    )


def test_campaign_terrain_layers_follow_shader_order() -> None:
    """Layer order is the fixed index contract of ``terrain.gdshader``."""
    assert [layer["id"] for layer in _document()["layers"]] == SHADER_ORDER


def test_campaign_terrain_layer_means_filled() -> None:
    """Layer means were written by the texture build (never left at zero)."""
    for layer in _document()["layers"]:
        assert sum(layer["mean_linear"]) > 0.0, layer["id"]


def test_water_normal_tiles_seamlessly() -> None:
    """The procedural sea normal wraps: edge step no larger than interior steps."""
    import numpy as np

    from cent_ans_tools.geo import textures

    normal = textures.water_normal(128).astype(np.int32)
    interior = np.abs(np.diff(normal, axis=1)).mean()
    edge = np.abs(normal[:, 0] - normal[:, -1]).mean()
    assert edge <= interior * 1.5
    interior_v = np.abs(np.diff(normal, axis=0)).mean()
    edge_v = np.abs(normal[0] - normal[-1]).mean()
    assert edge_v <= interior_v * 1.5
    assert normal[..., 2].min() > 128  # normals point up (z > 0)


def test_format_spec_round_trips() -> None:
    """``format_spec`` writes the same data it reads."""
    from cent_ans_tools.geo import textures

    spec = _document()
    assert json.loads(textures.format_spec(spec)) == spec
