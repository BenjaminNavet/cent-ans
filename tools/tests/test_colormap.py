"""Campaign ground colour map (chantier SS, lot SS1): painter, style, BC1 chain."""

from __future__ import annotations

import json
import zlib
from pathlib import Path

import numpy as np
import pytest
import yaml
from PIL import Image

from cent_ans_tools.codex import schema_validator
from cent_ans_tools.geo import block_compress, colormap

MPP = 718.9765625


@pytest.fixture(scope="module")
def style() -> dict:
    """The repository style (validated by :func:`colormap.load_style`)."""
    return colormap.load_style()


def _inputs(rows: int = 48, cols: int = 64, **overrides) -> colormap.ColormapInputs:
    """Flat farmland at 100 m, 47° N, everything else empty."""
    splat = np.zeros((rows, cols, 4), dtype=np.uint8)
    splat[..., 1] = 255
    values = {
        "land": np.ones((rows, cols), dtype=bool),
        "splat": splat,
        "height_m": np.full((rows, cols), 100.0, dtype=np.float32),
        "coast_dist_px": np.full((rows, cols), 60.0, dtype=np.float32),
        "wetlands": np.zeros((rows, cols, 3), dtype=np.uint8),
        "conifer": np.zeros((rows, cols), dtype=np.float32),
        "lon": np.full((rows, cols), 2.0, dtype=np.float32),
        "lat": np.full((rows, cols), 47.0, dtype=np.float32),
        "meters_per_px": MPP,
    }
    values.update(overrides)
    return colormap.ColormapInputs(**values)


def _distance(image: np.ndarray, color: str) -> np.ndarray:
    return np.abs(image.astype(np.float32) / 255.0 - colormap.hex_color(color)).sum(-1)


def test_repository_style_matches_schema() -> None:
    """``colormap_style.yaml`` validates against ``colormap_style.schema.json``."""
    validator = schema_validator(
        colormap.SCHEMA_PATH.parents[1], colormap.SCHEMA_PATH.name
    )
    document = yaml.safe_load(colormap.STYLE_PATH.read_text(encoding="utf-8"))
    errors = list(validator.iter_errors(document))
    assert not errors, [error.message for error in errors]


def test_invalid_style_is_rejected(tmp_path: Path) -> None:
    """A colour that is not ``#rrggbb`` fails validation."""
    document = yaml.safe_load(colormap.STYLE_PATH.read_text(encoding="utf-8"))
    document["palette"]["road"] = "beige"
    path = tmp_path / "style.yaml"
    path.write_text(yaml.safe_dump(document), encoding="utf-8")
    with pytest.raises(ValueError, match="palette/road"):
        colormap.load_style(path)


def test_bake_is_deterministic_and_band_independent(style: dict) -> None:
    """Same inputs, same bytes, whatever the band height."""
    settlements = np.array([[20.0, 20.0], [40.0, 30.0]])
    roads = [("main", np.array([[2.0, 10.0], [60.0, 14.0]]))]
    towns = [(30.0, 30.0, 40.0)]
    first = colormap.bake(
        _inputs(settlements=settlements, roads=roads, towns=towns), style, 32
    )
    second = colormap.bake(
        _inputs(settlements=settlements, roads=roads, towns=towns), style, 17
    )
    assert first.shape == (96, 128, 3)
    assert first.tobytes() == second.tobytes()


def test_road_is_painted_on_its_line(style: dict) -> None:
    """A horizontal road at y = 24.25 map px paints texture row 48 in dirt colour."""
    rows, cols = 48, 64
    road = [("main", np.array([[4.0, 24.25], [60.0, 24.25]]))]
    with_road = colormap.bake(_inputs(rows, cols, roads=road), style)
    without = colormap.bake(_inputs(rows, cols), style)
    color = style["palette"]["road"]
    on_line = _distance(with_road[48, 20:100], color)
    before = _distance(without[48, 20:100], color)
    far = _distance(with_road[20, 20:100], color)
    assert on_line.mean() < 0.5 * before.mean()
    assert on_line.mean() < 0.5 * far.mean()
    # Loin du tracé, rien ne change.
    assert np.array_equal(with_road[:40], without[:40])


def test_lake_has_a_shore_and_the_sea_does_not(style: dict) -> None:
    """Inland water gets a shore ring; water touching the map edge (sea) does not."""
    rows, cols = 48, 64
    land = np.ones((rows, cols), dtype=bool)
    land[:, :6] = False  # mer à l'ouest (touche le bord)
    land[20:28, 36:46] = False  # lac intérieur
    coast = np.full((rows, cols), 60.0, dtype=np.float32)
    coast[:, :12] = np.arange(12, dtype=np.float32) - 5.5
    image = colormap.bake(_inputs(rows, cols, land=land, coast_dist_px=coast), style)
    shore = style["palette"]["lake_shore"]
    # Anneau d'un texel autour du lac (texture = 2 x carte, lac en 40..56 x 72..92).
    ring = np.zeros((rows * 2, cols * 2), dtype=bool)
    ring[39:57, 71] = ring[39:57, 92] = True
    ring[39, 71:93] = ring[56, 71:93] = True
    sea_edge = np.zeros_like(ring)
    sea_edge[:, 12] = True
    assert _distance(image[ring], shore).mean() < 0.25
    assert _distance(image[sea_edge], shore).mean() > 0.35
    # L'eau du lac n'est pas peinte en terre.
    centre = image[46:50, 80:84]
    assert (
        _distance(centre, style["palette"]["lake_deep"]).mean()
        < _distance(centre, style["palette"]["farmland"]).mean()
    )


def test_modern_reservoir_is_painted_as_land() -> None:
    """A lake holding a ``modern_reservoirs`` point is not a lake."""
    land = np.ones((48, 64), dtype=bool)
    land[20:28, 36:46] = False
    lake, reservoir = colormap.classify_water(land, np.array([[40.0, 24.0]]), 10_000)
    assert not lake.any()
    assert reservoir.sum() == 80
    lake, reservoir = colormap.classify_water(land, np.zeros((0, 2)), 10_000)
    assert lake.sum() == 80 and not reservoir.any()


def test_farm_blocks_near_settlement_not_on_rock(style: dict) -> None:
    """Farmland near a village is a patchwork of crop colours; bare rock is not."""
    rows, cols = 64, 64
    settlements = np.array([[16.0, 32.0], [48.0, 32.0]])
    splat = np.zeros((rows, cols, 4), dtype=np.uint8)
    splat[:, :32, 1] = 255  # cultures à l'ouest
    splat[:, 32:, 3] = 255  # roche à l'est
    height = np.full((rows, cols), 100.0, dtype=np.float32)
    height[:, 32:] = 2000.0
    image = colormap.bake(
        _inputs(rows, cols, splat=splat, height_m=height, settlements=settlements),
        style,
    )
    mosaic = style["mosaic"]
    crops = [e["color"] for e in mosaic["crops"] + mosaic["pastures"]]

    def crop_share(patch: np.ndarray) -> float:
        nearest = np.min([_distance(patch, c) for c in crops], axis=0)
        return float((nearest < 0.12).mean())

    farm = image[40:88, 8:56]
    rock = image[40:88, 72:120]
    assert crop_share(farm) > 0.4
    assert crop_share(rock) < 0.05
    # Contraste de bloc à bloc sur les cultures, aplat sur la roche.
    farm_std = farm.reshape(-1, 3).std(axis=0).mean()
    rock_std = rock.reshape(-1, 3).std(axis=0).mean()
    assert farm_std > 3 * rock_std


def test_bc1_chain_size_matches_godot(tmp_path: Path) -> None:
    """BC1 + mipmaps has the size ``Image.create_from_data`` expects; parts add up."""
    rng = np.random.default_rng(3)
    image = rng.integers(0, 255, size=(40, 56, 3), dtype=np.uint8)
    (tmp_path / "map.json").write_text("{}", encoding="utf-8")
    paths, meta = colormap.write_colormap(image, tmp_path)
    expected = 0
    width, height = 56, 40
    while True:
        expected += ((width + 3) // 4) * ((height + 3) // 4) * 8
        if (width, height) == (1, 1):
            break
        width, height = max(1, width // 2), max(1, height // 2)
    assert meta["bytes"] == expected == sum(meta["part_bytes"])
    assert meta["format"] == "dxt1" and meta["mipmaps"] is True
    assert meta["size_px"] == [56, 40]
    raw = b"".join(zlib.decompress(p.read_bytes()) for p in paths)
    assert len(raw) == expected
    level0 = block_compress.decode_bc1(raw[: 14 * 10 * 8], 40, 56)
    assert level0.shape == (40, 56, 3)
    block_compress.update_map_json(tmp_path, colormap.MAP_KEY, meta)
    stored = json.loads((tmp_path / "map.json").read_text(encoding="utf-8"))
    assert stored["colormap"]["bc1"]["pattern"] == "colormap_bc1_{part}.bin"


def test_full_size_chain_bytes() -> None:
    """The real 14336 x 12288 texture: 8 bytes per 4 x 4 block over the whole chain."""
    assert colormap.mip_chain_bytes(14336, 12288, 8) == 117_440_544


def test_preview_width(tmp_path: Path) -> None:
    """The preview is resized to the requested width."""
    image = np.zeros((96, 128, 3), dtype=np.uint8)
    path = colormap.write_preview(image, tmp_path / "p.jpg", width=24)
    with Image.open(path) as preview:
        assert preview.size == (24, 18)
