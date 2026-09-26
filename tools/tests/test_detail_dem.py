"""Tests of the tier-3 relief pyramid (lot ZG3, ADR 0036): grid, footprints, sources, erasing, manifest."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
import pytest
import rasterio
import shapely
from rasterio.transform import Affine

from cent_ans_tools.geo import (
    anachronisms,
    detail_dem,
    detail_sources,
    relief_shade,
    terrain,
)
from cent_ans_tools.geo.detail_dem import PyramidGrid, Zone

DATA = Path(__file__).resolve().parents[2] / "data"
GRID = PyramidGrid(minx=2169486.0, maxy=4272786.0, unit_m=718.9765625)


def zone(
    zid: str = "z", lon: float = 1.4, lat: float = 49.2, half: float = 12.0, **kw
) -> Zone:
    """A synthetic zone."""
    return Zone(
        zid, zid, "battle", lon, lat, half, kw.pop("max_level", 7), "ign_rge_alti", **kw
    )


# ---------------------------------------------------------------------------- grid


def test_level_pixel_sizes() -> None:
    """E5-E7 are 11.23, 5.62 and 2.81 m, halving from E0 (359.5 m)."""
    assert GRID.pixel_m(0) == pytest.approx(359.488, abs=1e-3)
    assert GRID.pixel_m(5) == pytest.approx(11.234, abs=1e-3)
    assert GRID.pixel_m(7) == pytest.approx(2.8085, abs=1e-4)
    assert GRID.tiles_per_side(7) == 2048


def test_children_nest_in_parent() -> None:
    """Tile (k, c, r) and its four children share the parent's corners."""
    parent = GRID.transform(5, 100, 200)
    child = GRID.transform(6, 200, 400)
    assert parent.c == pytest.approx(child.c)
    assert parent.f == pytest.approx(child.f)
    far = GRID.transform(6, 202, 402)
    assert far.c == pytest.approx(parent.c + 512 * GRID.pixel_m(5))
    assert far.f == pytest.approx(parent.f - 512 * GRID.pixel_m(5))


def test_tile_range_covers_box() -> None:
    """The tiles returned meet the box and nothing more."""
    size = 512 * GRID.pixel_m(6)
    x0 = GRID.minx + 10.5 * size
    y1 = GRID.maxy - 20.25 * size
    col0, row0, col1, row1 = GRID.tile_range(
        6, (x0, y1 - 1.5 * size, x0 + 2.0 * size, y1)
    )
    assert (col0, row0, col1, row1) == (10, 20, 12, 21)


# ---------------------------------------------------------------------- footprints


def test_footprint_half_sizes_per_level() -> None:
    """E5 on half_size_km, E6 on min(half, 6), E7 on min(half, 3), overrides honoured."""
    big = zone(half=12.0)
    assert detail_dem.level_half_km(big, 5) == 12.0
    assert detail_dem.level_half_km(big, 6) == 6.0
    assert detail_dem.level_half_km(big, 7) == 3.0
    small = zone(half=2.0)
    assert detail_dem.level_half_km(small, 6) == 2.0
    capped = zone(half=8.0, max_level=6, level_half_km={6: 4.5})
    assert detail_dem.level_half_km(capped, 6) == 4.5
    assert detail_dem.level_half_km(capped, 7) is None
    box = detail_dem.footprint(big, 7)
    assert box[2] - box[0] == pytest.approx(6000.0)


def test_finer_footprints_nest_in_coarser_tiles() -> None:
    """Every E7 tile's parent is an E6 tile of the zone, every E6 tile's parent an E5 tile."""
    z = zone(half=9.0)
    for level in (6, 7):
        children = detail_dem.zone_tiles(GRID, z, level)
        parents = detail_dem.zone_tiles(GRID, z, level - 1)
        assert {(c // 2, r // 2) for c, r in children} <= parents


def test_clusters_merge_overlapping_zones() -> None:
    """Two zones 5 km apart share tiles at E5 and are baked together; far zones are not."""
    a = zone("a", lon=2.35, lat=48.85)
    b = zone("b", lon=2.43, lat=48.84, half=4.0)
    c = zone("c", lon=0.0, lat=45.0)
    groups = detail_dem.clusters(GRID, [a, b, c], 5)
    keys = sorted(g.key for g in groups)
    assert keys == ["E5_a+b", "E5_c"]


# ------------------------------------------------------------------------ sampling


def test_bilinear_reproduces_a_plane() -> None:
    """Bilinear x2 upsampling of a plane gives the plane at the fine pixel centres."""
    rows, cols = np.mgrid[0:16, 0:16].astype(np.float32)
    coarse = 3.0 * cols + 2.0 * rows  # value at coarse centre (j + 0.5)
    fine = detail_dem.bilinear(coarse, (16, 16), 2.0, 4.0, 4.0)
    fr, fc = np.mgrid[0:16, 0:16].astype(np.float32)
    expected = 3.0 * ((fc + 0.5) / 2 + 4 - 0.5) + 2.0 * ((fr + 0.5) / 2 + 4 - 0.5)
    assert np.allclose(fine, expected, atol=1e-4)


def test_ancestor_heights_from_coarser_tile(tmp_path: Path) -> None:
    """The ancestor of an E5 raster is the bilinear upsampling of the E4 tile (then E0)."""
    col4, row4 = 100, 120
    ramp = np.tile(np.linspace(10.0, 60.0, 512, dtype=np.float32), (512, 1))
    folder = tmp_path / "pyramid" / "E4"
    folder.mkdir(parents=True)
    terrain.write_png16(terrain.height_to_uint16(ramp), folder / f"{col4}_{row4}.png")
    result = detail_dem.ancestor_heights(
        GRID, 5, 2 * col4, 2 * row4, (512, 512), tmp_path
    )
    quantum = 5000.0 / 65535
    # Pixel 2j+1 of E5 sits at 3/4 of E4 pixel j -> j+1.
    step = (60.0 - 10.0) / 511
    assert result[10, 101] == pytest.approx(10.0 + step * 50.25, abs=2 * quantum)
    assert np.isfinite(result[1:-1, 1:-1]).all()


def test_source_reprojection_keeps_a_plane(tmp_path: Path) -> None:
    """A Lambert-93 chunk holding a plane lands on the EPSG:3035 grid unchanged."""
    from pyproj import Transformer

    z = zone(lon=1.40, lat=49.24, half=1.0)
    cluster = detail_dem.clusters(GRID, [z], 6)[0]
    to_l93 = Transformer.from_crs("EPSG:3035", "EPSG:2154", always_xy=True)
    to_3035 = Transformer.from_crs("EPSG:2154", "EPSG:3035", always_xy=True)
    cx, cy = z.center_3035
    ex, ey = to_l93.transform(cx, cy)
    pixel = 2.0
    size = 2500
    west, north = ex - size, ey + size
    cols, rows = np.meshgrid(np.arange(size) + 0.5, np.arange(size) + 0.5)
    x3035, y3035 = to_3035.transform(west + cols * pixel, north - rows * pixel)
    plane = (0.01 * (x3035 - cx) + 0.02 * (y3035 - cy) + 100.0).astype(np.float32)
    path = tmp_path / "chunk.tif"
    detail_sources.write_chunk(
        plane, Affine(pixel, 0, west, 0, -pixel, north), "EPSG:2154", path
    )
    heights = detail_dem.source_heights(cluster, GRID, {z.id: [("ign_rge_alti", path)]})
    transform = GRID.transform(6, cluster.col0, cluster.row0)
    r, c = np.nonzero(np.isfinite(heights))
    assert len(r) > 1000
    px, py = transform * (c + 0.5, r + 0.5)
    expected = 0.01 * (np.asarray(px) - cx) + 0.02 * (np.asarray(py) - cy) + 100.0
    inner = (np.abs(np.asarray(px) - cx) < 1500) & (np.abs(np.asarray(py) - cy) < 1500)
    assert np.abs(heights[r, c][inner] - expected[inner]).max() < 0.05


# -------------------------------------------------------------------------- blend


def test_footprint_weight_is_zero_on_the_edge() -> None:
    """The fade reaches 0 on the footprint edge and 1 in its centre."""
    z = zone(half=4.0)
    cluster = detail_dem.clusters(GRID, [z], 5)[0]
    weight = detail_dem.footprint_weight(GRID, cluster)
    transform = GRID.transform(5, cluster.col0, cluster.row0)
    cx, cy = z.center_3035
    col, row = ~transform * (cx, cy)
    assert weight[int(row), int(col)] == pytest.approx(1.0)
    col_edge, _ = ~transform * (cx + 4000.0, cy)
    assert weight[int(row), int(col_edge) + 1] == 0.0
    assert weight[0, 0] == 0.0


def test_blend_uses_ancestor_where_fine_is_missing() -> None:
    """No data (sea) -> ancestor; full weight -> fine."""
    fine = np.array([[np.nan, 5.0]], dtype=np.float32)
    ancestor = np.array([[-3.0, 1.0]], dtype=np.float32)
    weight = np.array([[1.0, 1.0]], dtype=np.float32)
    assert detail_dem.blend(fine, ancestor, weight).tolist() == [[-3.0, 5.0]]


def test_boost_matches_relief_shade() -> None:
    """With the same base, the boost equals relief_shade.boost_relief's formula."""
    height = np.full((4, 4), 150.0, dtype=np.float32)
    base = np.full((4, 4), 100.0, dtype=np.float32)
    boosted = detail_dem.apply_boost(height, base)
    assert boosted[0, 0] == pytest.approx(150.0 + 0.8 * 50.0)
    sea = detail_dem.apply_boost(np.full((1, 1), -5.0, np.float32), base[:1, :1])
    assert sea[0, 0] == -5.0


def test_apply_boost_never_sinks_land_below_sea_level() -> None:
    """ZG3b regression: a high base never drags land below MIN_LAND_M.

    Mirrors relief_shade.enforce_coast. The London bug baked -11 to -15 m
    over real land at ~4 m before this floor existed.
    """
    height = np.full((1, 1), 0.55, dtype=np.float32)
    base = np.full((1, 1), 23.0, dtype=np.float32)  # observed Southwark base, E7
    boosted = detail_dem.apply_boost(height, base)
    assert boosted[0, 0] == pytest.approx(detail_dem.MIN_LAND_M)
    assert boosted[0, 0] >= detail_dem.MIN_LAND_M


def test_apply_boost_keeps_low_banks_above_the_water() -> None:
    """ZG7a/SZ2: London's low banks (2-5 m ODN) keep most of their height.

    Before ZG7a the flat 0.5 m floor put Southwark, Lambeth and Westminster level
    with the Thames. The floor (shared with E0-E4 since SZ2) is monotone.
    """
    base = np.full((1, 4), 23.0, dtype=np.float32)
    height = np.array([[2.0, 3.5, 5.0, 20.0]], dtype=np.float32)
    boosted = detail_dem.apply_boost(height, base)[0]
    keep = relief_shade.VALLEY_KEEP
    assert boosted[0] == pytest.approx(2.0 * keep)
    assert boosted[1] >= 2.9 and boosted[2] >= 4.2
    assert boosted[3] == pytest.approx(20.0 - relief_shade.VALLEY_DIG_MAX_M)
    assert all(boosted[i] <= boosted[i + 1] for i in range(3))
    # Land the boost raises is untouched by the floor.
    hills = detail_dem.apply_boost(np.full((1, 1), 80.0, np.float32), base[:, :1])
    assert hills[0, 0] == pytest.approx(80.0 + 0.8 * 57.0)


def test_boost_base_ignores_glo90_when_fine_data_exists(monkeypatch) -> None:
    """ZG3b regression: a small footprint's base comes from its own fine data.

    Not from GLO-90 leaking in through the 5 km blur: the footprint (3-6 km
    half) is far smaller than 3 sigma (15 km, BASE_MARGIN_M), so the old
    inner-overlay-then-blur base barely differed from raw GLO-90.
    """

    def _boom(*args, **kwargs):  # noqa: ANN001, ANN002, ANN003
        raise AssertionError("GLO-90 must not be read when fine data covers the raster")

    monkeypatch.setattr(detail_dem.copernicus, "resample_to_grid", _boom)
    z = zone(half=3.0)
    cluster = detail_dem.clusters(GRID, [z], 7)[0]
    fine = np.full(cluster.shape, 4.0, dtype=np.float32)
    base = detail_dem.boost_base(GRID, cluster, fine)
    assert np.allclose(base, 4.0, atol=0.05)


def test_boost_base_falls_back_to_glo90_without_any_fine_data(monkeypatch) -> None:
    """A cluster with no fine data at all still falls back to blurred GLO-90.

    As before this fix -- only the normal, fine-covered case changed.
    """
    calls = []

    def _fake_resample(map_grid, window, names):  # noqa: ANN001
        calls.append(window)
        rows, cols = window[3], window[2]
        return np.full((rows, cols), 42.0, dtype=np.float32)

    monkeypatch.setattr(detail_dem.copernicus, "resample_to_grid", _fake_resample)
    z = zone(half=3.0)
    cluster = detail_dem.clusters(GRID, [z], 7)[0]
    fine = np.full(cluster.shape, np.nan, dtype=np.float32)
    base = detail_dem.boost_base(GRID, cluster, fine)
    assert calls  # GLO-90 was consulted
    assert np.allclose(base, 42.0, atol=0.05)


def _write_flat_level(
    tmp_path: Path,
    level: int,
    col0: int,
    row0: int,
    col1: int,
    row1: int,
    height: float,
) -> None:
    """Write flat tiles of ``height`` covering ``[col0, col1] x [row0, row1]`` at ``level``."""
    for row in range(row0, row1 + 1):
        for col in range(col0, col1 + 1):
            path = tmp_path / "pyramid" / f"E{level}" / f"{col}_{row}.png"
            path.parent.mkdir(parents=True, exist_ok=True)
            terrain.write_png16(
                terrain.height_to_uint16(np.full((512, 512), height, np.float32)), path
            )


def test_e5_e4_land_gap_zero_on_flat_matching_land(tmp_path: Path) -> None:
    """Identical flat E5/E4 land gives a zero gap (a healthy bake)."""
    z = zone(half=1.0)
    col0, row0, col1, row1 = GRID.tile_range(5, detail_dem.footprint(z, 5))
    _write_flat_level(tmp_path, 4, col0 // 2, row0 // 2, col1 // 2, row1 // 2, 50.0)
    _write_flat_level(tmp_path, 5, col0, row0, col1, row1, 50.0)
    gap = detail_dem.e5_e4_land_gap(z, GRID, tmp_path)
    assert gap.n_pixels > 0
    assert gap.median_m == pytest.approx(0.0, abs=0.01)
    assert gap.p95_m < 0.5


def test_e5_e4_land_gap_excludes_water_below_min_land_m(tmp_path: Path) -> None:
    """A -11 m E5 (the raw Southwark symptom) reads as water, not land: excluded, not flagged."""
    z = zone(half=1.0)
    col0, row0, col1, row1 = GRID.tile_range(5, detail_dem.footprint(z, 5))
    _write_flat_level(tmp_path, 4, col0 // 2, row0 // 2, col1 // 2, row1 // 2, 4.0)
    _write_flat_level(
        tmp_path, 5, col0, row0, col1, row1, -11.0
    )  # observed at Southwark
    gap = detail_dem.e5_e4_land_gap(z, GRID, tmp_path)
    assert gap.n_pixels == 0


def test_e5_e4_land_gap_flags_a_boost_leak(tmp_path: Path) -> None:
    """A large, uniform E5 undershoot that stays above MIN_LAND_M is flagged."""
    z = zone(half=1.0)
    col0, row0, col1, row1 = GRID.tile_range(5, detail_dem.footprint(z, 5))
    _write_flat_level(tmp_path, 4, col0 // 2, row0 // 2, col1 // 2, row1 // 2, 50.0)
    _write_flat_level(tmp_path, 5, col0, row0, col1, row1, 1.0)  # still land, 49 m off
    gap = detail_dem.e5_e4_land_gap(z, GRID, tmp_path)
    assert gap.n_pixels > 0
    assert gap.p95_m > detail_dem.LAND_GAP_ALERT_M


def test_e5_e4_land_gap_no_tiles_returns_none(tmp_path: Path) -> None:
    """A zone with no E5 tiles baked yet reports ``None`` stats, not a crash."""
    z = zone(half=1.0)
    gap = detail_dem.e5_e4_land_gap(z, GRID, tmp_path)
    assert gap.median_m is None
    assert gap.n_pixels == 0


# ----------------------------------------------------------------------- erasing


def test_laplace_fill_recovers_a_plane_under_an_embankment() -> None:
    """An embankment on a sloping plane is erased back to the plane."""
    rows, cols = np.mgrid[0:60, 0:80].astype(np.float32)
    plane = 50.0 + 0.3 * cols - 0.2 * rows
    dem = plane.copy()
    dem[25:35, 8:72] += 6.0  # motorway embankment
    mask = np.zeros(dem.shape, dtype=bool)
    mask[23:37, 5:75] = True
    filled = anachronisms.laplace_fill(dem, mask)
    assert np.abs(filled - plane).max() < 1e-3


def test_laplace_fill_keeps_unmasked_relief() -> None:
    """Pixels outside the mask are untouched, NaN borders are free."""
    dem = np.random.default_rng(1).normal(100.0, 5.0, (40, 40)).astype(np.float32)
    dem[:, :3] = np.nan
    mask = np.zeros(dem.shape, dtype=bool)
    mask[10:20, 2:30] = True
    filled = anachronisms.laplace_fill(dem, mask & np.isfinite(dem))
    assert np.array_equal(filled[~mask], dem[~mask], equal_nan=True)
    assert np.isfinite(filled[10:20, 3:30]).all()


def test_fill_small_holes_only() -> None:
    """A pond inside is filled, the sea touching the edge stays NaN."""
    dem = np.full((50, 50), 10.0, dtype=np.float32)
    dem[20:24, 20:24] = np.nan
    dem[:, 45:] = np.nan
    filled = anachronisms.fill_small_holes(dem, max_px=100)
    assert np.isfinite(filled[20:24, 20:24]).all()
    assert np.isnan(filled[:, 45:]).all()


def test_modern_mask_buffers_a_motorway() -> None:
    """A motorway line becomes a band of about twice its buffer width."""
    from pyproj import Transformer

    to_geo = Transformer.from_crs("EPSG:3035", "EPSG:4326", always_xy=True)
    x0, y0 = 3700000.0, 2900000.0
    lon, lat = to_geo.transform([x0, x0 + 2000.0], [y0 + 500.0, y0 + 500.0])
    shapes = [(shapely.LineString(list(zip(lon, lat, strict=True))), "motorway")]
    transform = Affine(5.0, 0.0, x0, 0.0, -5.0, y0 + 1000.0)
    mask = anachronisms.modern_mask(shapes, transform, (200, 400), dilate_px=0)
    column = mask[:, 200]
    width_m = column.sum() * 5.0
    assert (
        2 * anachronisms.BUFFER_M["motorway"] - 10
        <= width_m
        <= 2 * anachronisms.BUFFER_M["motorway"] + 10
    )


def test_osm_filter_skips_bridges_and_tunnels() -> None:
    """Bridges, tunnels and ordinary roads are kept; quarries and railways are erased."""
    assert anachronisms.feature_class({"highway": "motorway", "bridge": "yes"}) is None
    assert anachronisms.feature_class({"railway": "rail", "tunnel": "yes"}) is None
    assert anachronisms.feature_class({"highway": "primary"}) is None
    assert anachronisms.feature_class({"railway": "rail"}) == "rail"
    assert anachronisms.feature_class({"landuse": "quarry"}) == "area"
    payload = {
        "elements": [
            {
                "type": "way",
                "tags": {"landuse": "quarry"},
                "geometry": [
                    {"lon": 1.0, "lat": 49.0},
                    {"lon": 1.01, "lat": 49.0},
                    {"lon": 1.01, "lat": 49.01},
                    {"lon": 1.0, "lat": 49.0},
                ],
            }
        ]
    }
    shapes = anachronisms.osm_shapes(payload)
    assert len(shapes) == 1 and shapes[0][0].geom_type == "Polygon"


# ------------------------------------------------------------------------ sources


def test_plan_chunks_tile_the_box() -> None:
    """Chunks never exceed max_px and tile the snapped box exactly."""
    source = detail_sources.SOURCES["ign_rge_alti"]
    chunks = detail_sources.plan_chunks(
        source, (600000.0, 6800000.0, 626000.0, 6812000.0), 5.0
    )
    assert all(w <= source.max_px and h <= source.max_px for _, (w, h) in chunks)
    area = sum((b[2] - b[0]) * (b[3] - b[1]) for b, _ in chunks)
    assert area == pytest.approx(26000.0 * 12000.0)


def test_clean_heights_drops_every_nodata_flavour() -> None:
    """±3.4e38, -9999 and -99999 all become NaN."""
    raw = np.array([3.4e38, -3.4e38, -9999.0, -99999.0, 12.5], dtype=np.float32)
    clean = detail_sources.clean_heights(raw, -99999.0)
    assert np.isnan(clean[:4]).all() and clean[4] == 12.5


def test_multipart_answer_yields_the_tiff() -> None:
    """The Flemish WCS answers GML + TIFF in one body."""
    body = b"--wcs\nContent-Type: text/xml\n\n<xml/>\n--wcs\nContent-Type: image/tiff\n\nII*\x00rest"
    assert detail_sources.extract_tiff(body) == b"II*\x00rest"


def test_written_chunk_round_trips(tmp_path: Path) -> None:
    """Cached chunks keep heights to 1/64 m and NaN."""
    data = np.array([[1.234, np.nan], [250.5, -3.0]], dtype=np.float32)
    path = tmp_path / "c.tif"
    detail_sources.write_chunk(data, Affine(1, 0, 0, 0, -1, 2), "EPSG:2154", path)
    with rasterio.open(path) as dataset:
        back = dataset.read(1)
    assert np.isnan(back[0, 1])
    assert np.allclose(back[~np.isnan(back)], data[~np.isnan(data)], atol=1 / 128)


# ----------------------------------------------------------------------- manifest


def test_tiles_rle() -> None:
    """Rows of [col_start, length] runs."""
    rle = detail_dem.tiles_rle({(3, 1), (4, 1), (6, 1), (5, 0)})
    assert rle == [{"row": 0, "runs": [[5, 1]]}, {"row": 1, "runs": [[3, 2], [6, 1]]}]


def test_manifest_update_touches_only_its_lines(tmp_path: Path) -> None:
    """Levels 5-7 lines are replaced, every other byte is kept."""
    original = (DATA / "map" / "relief_pyramid.json").read_text(encoding="utf-8")
    path = tmp_path / "relief_pyramid.json"
    path.write_text(original, encoding="utf-8")
    lines = {
        level: detail_dem.level_line(level, GRID, {(10, 20), (11, 20)}, None)
        for level in (5, 6, 7)
    }
    detail_dem.update_manifest(lines, path)
    updated = path.read_text(encoding="utf-8")
    old_lines, new_lines = original.split("\n"), updated.split("\n")
    assert len(old_lines) == len(new_lines)
    changed = [
        i for i, (a, b) in enumerate(zip(old_lines, new_lines, strict=True)) if a != b
    ]
    for index in changed:
        assert (
            '"level": 5' in old_lines[index]
            or '"level": 6' in old_lines[index]
            or '"level": 7' in old_lines[index]
        )
    manifest = json.loads(updated)
    level7 = next(level for level in manifest["levels"] if level["level"] == 7)
    assert level7["tiles_rle"] == [{"row": 20, "runs": [[10, 2]]}]
    for number in (5, 6, 7):
        line = next(line for line in new_lines if f'"level": {number},' in line)
        assert line.strip().startswith("{ ") and line.rstrip(",").endswith("}")


# -------------------------------------------------------------------------- zones


def test_zones_are_consistent() -> None:
    """Unique ids, known sources, existing settlements, ~25-35 zones including the 7 cities."""
    document = json.loads(
        (DATA / "map" / "detail_zones.json").read_text(encoding="utf-8")
    )
    zones = document["zones"]
    ids = [z["id"] for z in zones]
    assert len(ids) == len(set(ids))
    assert 25 <= len(zones) <= 40
    settlements = set()
    for path in (DATA / "settlements").glob("prov_*.json"):
        settlements.update(
            s["id"] for s in json.loads(path.read_text(encoding="utf-8"))
        )
    landmark_settlements = {
        json.loads(p.read_text(encoding="utf-8"))["settlement"]
        for p in (DATA / "landmarks").glob("*.json")
    }
    linked = {z.get("settlement_id") for z in zones}
    assert landmark_settlements <= linked
    for z in zones:
        if "settlement_id" in z:
            assert z["settlement_id"] in settlements, z["id"]
        for key in [z["source"], *z.get("extra_sources", [])]:
            assert key in detail_sources.SOURCES, (z["id"], key)
