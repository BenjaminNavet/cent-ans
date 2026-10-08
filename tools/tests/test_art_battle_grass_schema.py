"""Validates the FA7 battle grass catalogue against its schema and the built atlas."""

import json
from pathlib import Path

import numpy as np
from PIL import Image

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
ATLAS = ROOT / "game/assets/textures/battle/grass_tufts.png"


def _document() -> dict:
    return json.loads((DATA / "art" / "battle_grass.json").read_text(encoding="utf-8"))


def test_battle_grass_references_resolve() -> None:
    """Layers, flowers and field variants name catalogued entries; weights are usable."""
    document = _document()
    names = [variant["name"] for variant in document["variants"]]
    assert len(names) == len(set(names))
    atlas = document["atlas"]
    assert len(names) <= atlas["columns"] * atlas["rows"]
    for variant in document["variants"]:
        for layer in variant["layers"]:
            assert layer["source"] in document["sources"], layer["source"]
            assert layer["length_m"][0] <= layer["length_m"][1]
        for flower in variant.get("flowers", []):
            assert flower["kind"] in document["flowers"], flower["kind"]
    fields = document["render"]["fields"]
    for name in fields["wheat_variants"] + fields["stubble_variants"]:
        assert name in names, name
    for key in ("weight_open", "weight_clump"):
        assert sum(variant[key] for variant in document["variants"]) > 0


def test_battle_grass_atlas_matches_catalogue() -> None:
    """The built atlas has the catalogued grid, a tuft in every cell and the target luminance."""
    document = _document()
    atlas = document["atlas"]
    image = Image.open(ATLAS)
    assert image.mode == "RGBA"
    assert image.size == (
        atlas["columns"] * atlas["cell_width"],
        atlas["rows"] * atlas["cell_height"],
    )
    pixels = np.asarray(image).astype(np.float64) / 255.0
    alpha = pixels[..., 3]
    for index, variant in enumerate(document["variants"]):
        x = (index % atlas["columns"]) * atlas["cell_width"]
        y = (index // atlas["columns"]) * atlas["cell_height"]
        cell = alpha[y : y + atlas["cell_height"], x : x + atlas["cell_width"]]
        coverage = float((cell > 0.5).mean())
        assert 0.04 < coverage < 0.6, (variant["name"], coverage)
        # Margins stay empty on the sides and the top: no bleeding between tufts.
        pad = atlas["padding"]
        if pad > 1:
            assert cell[: pad - 1].max() < 0.05, variant["name"]
            assert cell[:-pad, : pad - 1].max() < 0.05, variant["name"]
    srgb = pixels[..., :3]
    linear = np.where(srgb <= 0.04045, srgb / 12.92, ((srgb + 0.055) / 1.055) ** 2.4)
    luma = (linear @ np.array([0.3, 0.59, 0.11]) * alpha).sum() / alpha.sum()
    assert abs(luma - document["render"]["tex_lum"]) < 0.01, luma
