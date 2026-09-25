"""Tests for the lot R1 rasters: Copernicus tiles, render relief, historical land cover."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator

from cent_ans_tools.geo import copernicus, kk10, landcover, relief_shade
from cent_ans_tools.geo.project import MapGrid

DATA = Path(__file__).resolve().parents[2] / "data"


def test_copernicus_tile_names_round_trip_and_bbox_filter() -> None:
    """Names follow the bucket convention (south-west corner) and are filtered by the bbox."""
    name = copernicus.tile_name(-2, 48)
    assert name == "Copernicus_DSM_COG_30_N48_00_W002_00_DEM"
    assert copernicus.parse_tile_name(name) == (-2, 48)
    names = [name, copernicus.tile_name(20, 48), copernicus.tile_name(5, 38), "junk"]
    assert copernicus.tiles_in_bbox(names, (-11.0, 41.0, 12.0, 60.0)) == [name]


def test_merge_keeps_etopo_at_sea_and_without_tiles() -> None:
    """Copernicus wins on land; its 0 m sea surface and NaN gaps take the ETOPO value."""
    cop = np.array([[120.0, 0.0, np.nan]], dtype=np.float32)
    etopo = np.array([[100.0, -40.0, 55.0]], dtype=np.float32)
    np.testing.assert_allclose(
        copernicus.merge_with_etopo(cop, etopo), [[120.0, -40.0, 55.0]]
    )


def test_boost_amplifies_local_relief_only_in_lowlands_on_land() -> None:
    """A small hill in a plain grows; the sea and high mountains are untouched."""
    size = 64
    height = np.full((size, size), 100.0, dtype=np.float32)
    height[30:34, 30:34] = 160.0
    land = np.ones((size, size), dtype=bool)
    land[:, :4] = False
    boosted = relief_shade.boost_relief(height, land, 1000.0)
    assert boosted[31, 31] > 160.0 + 20.0
    assert (
        boosted[31, 31] - 160.0
        <= relief_shade.BOOST_GAIN * relief_shade.BOOST_LIMIT_M + 1e-3
    )
    np.testing.assert_array_equal(boosted[:, :4], height[:, :4])
    alps = height + 3000.0
    np.testing.assert_allclose(
        relief_shade.boost_relief(alps, land, 1000.0), alps, atol=1e-2
    )


def test_enforce_coast_keeps_the_shoreline() -> None:
    """Land stays above sea level and sea at or below it, whatever the boost did."""
    height = np.array([[-5.0, 3.0], [12.0, -1.0]], dtype=np.float32)
    land = np.array([[True, True], [False, False]])
    out = relief_shade.enforce_coast(height, land)
    assert out[0, 0] == relief_shade.MIN_LAND_M
    assert out[0, 1] == 3.0
    assert out[1, 0] == 0.0
    assert out[1, 1] == -1.0


def test_block_mean_and_bilinear_upsample_are_consistent() -> None:
    """A linear ramp survives downsampling then GPU-style bilinear upsampling."""
    fine = np.tile(np.arange(16, dtype=np.float32), (16, 1))
    coarse = relief_shade.block_mean(fine, 2)
    assert coarse.shape == (8, 8)
    up = relief_shade.bilinear_upsample(coarse, 2)
    assert up.shape == (16, 16)
    np.testing.assert_allclose(up[:, 2:-2], fine[:, 2:-2], atol=1e-4)


def test_shade_encoding_is_neutral_at_128() -> None:
    """Zero detail and zero curvature encode as 128 in both channels."""
    enc = relief_shade.encode_shade(np.zeros((2, 2)), np.zeros((2, 2)))
    assert enc.shape == (2, 2, 2)
    assert np.all(enc == 128)


def test_ellipse_signed_distance_is_positive_inside() -> None:
    """The centre of an ellipse is inside (positive), a far pixel outside (negative)."""
    grid = MapGrid((2169486.0, 1327858.0, 5114414.0, 4272786.0), 4096)
    area = {"ellipse": {"center": [2.18, 47.98], "radii_km": [30, 10], "angle_deg": 0}}
    dist, window = landcover.area_signed_distance(area, grid)
    cx, cy = grid.lonlat_to_pixel(2.18, 47.98)
    row = int(cy) - window[0].start
    col = int(cx) - window[1].start
    assert dist[row, col] > 10.0
    assert dist[0, 0] < 0.0
    # Grand axe est-ouest : à 20 px à l'est on reste dedans, à 20 px au nord on en sort.
    assert dist[row, col + 20] > 0.0
    assert dist[row - 20, col] < 0.0


def test_rank_uniform_is_uniform_on_the_mask() -> None:
    """Ranks spread evenly in [0, 1] over the masked pixels, 0 elsewhere."""
    values = np.arange(10, dtype=np.float32).reshape(2, 5)[::-1]
    mask = np.ones_like(values, dtype=bool)
    mask[0, 0] = False
    ranks = landcover.rank_uniform(values, mask)
    assert ranks[0, 0] == 0.0
    assert np.isclose(ranks[mask].min(), 0.0)
    assert np.isclose(ranks[mask].max(), 1.0)


def test_compose_splat_keeps_the_contract() -> None:
    """R/G/B/A sum to 1 on land, B is the forest cover, 0 at sea."""
    base = np.zeros((4, 4, 4), dtype=np.float32)
    base[1:, :, :] = [0.4, 0.5, 0.1, 0.0]
    forest = np.full((4, 4), 0.3, dtype=np.float32)
    zeros = np.zeros((4, 4), dtype=np.float32)
    weights = landcover.compose_splat(
        base, forest, zeros, np.zeros((4, 4, 3), dtype=np.float32)
    )
    np.testing.assert_allclose(weights[1:].sum(axis=-1), 1.0, atol=1e-5)
    np.testing.assert_allclose(weights[1:, :, 2], 0.3, atol=1e-6)
    assert np.all(weights[0] == 0.0)


def test_tree_line_falls_with_latitude() -> None:
    """About 2 000 m in the Alps, well under 1 000 m in Scotland."""
    lines = landcover.tree_line_m(np.array([44.0, 57.0]))
    assert 1900.0 <= lines[0] <= 2150.0
    assert lines[1] < 800.0


def test_kk10_load_returns_none_without_cache(tmp_path: Path) -> None:
    """No extraction yet: the land cover falls back to a uniform cleared share."""
    assert kk10.load(tmp_path / "missing.npz") is None
    grid = MapGrid((0.0, 0.0, 1000.0, 1000.0), 8)
    assert np.all(landcover.kk10_cleared(grid, None) == 0.6)


def test_curated_files_match_their_schemas() -> None:
    """historical_forests.json and wetlands.json validate; ids are unique."""
    for name in ("historical_forests", "wetlands"):
        schema = json.loads(
            (DATA / "schemas" / f"{name}.schema.json").read_text(encoding="utf-8")
        )
        document = json.loads(
            (DATA / "map" / f"{name}.json").read_text(encoding="utf-8")
        )
        errors = list(Draft202012Validator(schema).iter_errors(document))
        assert not errors, errors[:3]
        ids = [area["id"] for area in document["areas"]]
        assert len(ids) == len(set(ids))
