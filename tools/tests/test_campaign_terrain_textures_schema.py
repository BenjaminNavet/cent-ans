"""Validates the campaign terrain texture data against its schema (lot GA4)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"
SHADER_ORDER = ["grass", "farmland", "forest", "rock", "heath", "snow", "sand"]


def _document() -> dict:
    return json.loads(
        (DATA / "fx" / "campaign_terrain_textures.json").read_text(encoding="utf-8")
    )


def test_campaign_terrain_has_no_poly_haven_layers() -> None:
    """ADR 0244: the global GA4 Poly Haven layers are gone; the regional block is the only path."""
    document = _document()
    assert "layers" not in document
    assert "poly_haven" not in json.dumps(document)
    assert set(document["regional"]["roles"]) == set(SHADER_ORDER)


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
