"""Texture factory processing stages (T1b) on small synthetic images."""

from __future__ import annotations

import json

import numpy as np
from PIL import Image

from cent_ans_tools.texture_factory.board import board
from cent_ans_tools.texture_factory.checks import (
    blotch_score,
    luminance_in_range,
    saturation_in_range,
    seam_ratio,
)
from cent_ans_tools.texture_factory.color import linear_to_srgb, srgb_to_linear
from cent_ans_tools.texture_factory.pack import grid_shape, import_file, pack
from cent_ans_tools.texture_factory.pbr import (
    derive_normal_rough,
    equalize_luminance,
    flatten_lighting,
)
from cent_ans_tools.texture_factory.seamless import low_pass, make_seamless


def _noise(size: int = 128, seed: int = 1) -> np.ndarray:
    rng = np.random.default_rng(seed)
    return low_pass(rng.random((size, size, 3)), 2.0, "reflect")[:, :]


def test_color_roundtrip() -> None:
    """Stage check."""
    values = np.linspace(0, 1, 50)
    assert np.allclose(linear_to_srgb(srgb_to_linear(values)), values, atol=1e-6)


def test_seamless_reduces_seam() -> None:
    """Stage check."""
    gradient = np.linspace(0, 1, 128)[None, :, None] * np.ones((128, 128, 3))
    image = gradient + 0.05 * _noise()
    assert seam_ratio(image) > 3.0
    assert seam_ratio(make_seamless(image, half_band=24)) < 1.5


def test_pbr_stages() -> None:
    """Stage check."""
    linear = np.clip(_noise(), 0.01, 1.0)
    assert (
        abs(
            float((equalize_luminance(linear, 0.2) @ [0.2126, 0.7152, 0.0722]).mean())
            - 0.2
        )
        < 0.01
    )
    stained = linear.copy()
    stained[:64, :64] *= 0.3
    assert blotch_score(flatten_lighting(stained, 32)) < blotch_score(stained)
    packed = derive_normal_rough(linear, 2.0, 0.8)
    assert packed.shape == (128, 128, 3) and packed.dtype == np.uint8


def test_checks_ranges() -> None:
    """Stage check."""
    grey = np.full((8, 8, 3), 128, dtype=np.uint8)
    assert luminance_in_range(grey, 0.4, 0.6)
    assert not luminance_in_range(grey, 0.7, 0.9)
    assert saturation_in_range(grey, 0.0, 0.05)
    red = np.zeros((8, 8, 3), dtype=np.uint8)
    red[..., 0] = 200
    assert saturation_in_range(red, 0.9, 1.0)


def test_pack_and_board(tmp_path) -> None:
    """Stage check."""
    tiles = tmp_path / "raw" / "tiles"
    tiles.mkdir(parents=True)
    materials = []
    for index in range(4):
        materials.append(
            {
                "id": f"m{index}",
                "layer": index,
                "role": "ground",
                "biomes": ["x"],
                "tile_m": 2.0,
            }
        )
        Image.new("RGB", (16, 16), (60 * index, 80, 40)).save(
            tiles / f"m{index}_albedo.png"
        )
        Image.new("RGB", (16, 16), (128, 128, 255)).save(tiles / f"m{index}_normal.png")
    document = {"layer_size": 16, "materials": materials}
    assert grid_shape(4) == (2, 2)
    assert "slices/horizontal=2" in import_file(2, 2, "a.jpg", "res://x/")
    manifest = tmp_path / "pack.json"
    report = pack(
        document,
        tmp_path / "raw",
        tmp_path / "tex",
        manifest,
        albedo_name="a.jpg",
        normal_name="n.jpg",
        res_dir="res://x/",
    )
    assert report["grid"] == [2, 2]
    assert json.loads(manifest.read_text())["albedo"] == "res://x/a.jpg"
    assert (tmp_path / "tex" / "n.jpg.import").exists()
    target = board(document, tmp_path / "raw", cell=64)
    assert target.exists()
