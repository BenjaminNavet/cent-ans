"""Relief pyramid tiers 1-2 (lot ZG1, ADR 0036): geometry, RLE, manifest, corrections."""

import json
import shutil
from pathlib import Path

import numpy as np
import pytest
from jsonschema import Draft202012Validator

from cent_ans_tools.geo import pyramid, relief_shade, surface, terrain

DATA = Path(__file__).resolve().parents[2] / "data"
MAP_DIR = DATA / "map"
BOUNDS = (2169486.0, 1327858.0, 5114414.0, 4272786.0)


# ------------------------------------------------------------------------ geometry


def test_levels_halve_pixel_size() -> None:
    """Level k is an 8192 * 2^k grid: 359.49 m at E0, halved at each level."""
    assert pyramid.level_size_px(0) == 8192
    assert pyramid.level_tiles(4) == 256
    assert pyramid.level_meters_per_px(BOUNDS, 0) == pytest.approx(359.488, abs=1e-3)
    for level in range(1, 8):
        assert pyramid.level_meters_per_px(BOUNDS, level) == pytest.approx(
            pyramid.level_meters_per_px(BOUNDS, level - 1) / 2
        )


def test_children_tile_their_parent_exactly() -> None:
    """The four children of a tile cover it exactly, and parent() inverts children()."""
    key = pyramid.TileKey(2, 37, 21)
    minx, miny, maxx, maxy = pyramid.tile_bounds(BOUNDS, key)
    children = key.children()
    assert {child.parent() for child in children} == {key}
    boxes = [pyramid.tile_bounds(BOUNDS, child) for child in children]
    assert min(b[0] for b in boxes) == pytest.approx(minx)
    assert max(b[2] for b in boxes) == pytest.approx(maxx)
    assert min(b[1] for b in boxes) == pytest.approx(miny)
    assert max(b[3] for b in boxes) == pytest.approx(maxy)
    area = sum((b[2] - b[0]) * (b[3] - b[1]) for b in boxes)
    assert area == pytest.approx((maxx - minx) * (maxy - miny))
    # One tile is 256 / 2^k world units of 718.98 m.
    assert maxx - minx == pytest.approx(256 / 4 * 718.9765625)


def test_e0_coordinates_are_pixel_centred() -> None:
    """Two level-1 pixels straddle the centre of their E0 pixel symmetrically."""
    coords = pyramid.e0_coordinates(1, 0, 4)
    assert coords.tolist() == pytest.approx([-0.25, 0.25, 0.75, 1.25])
    # Nearest sampling of a level-4 window returns the covering E0 pixel.
    e0 = np.arange(64, dtype=np.float32).reshape(8, 8)
    nearest = pyramid.sample_e0_grid(e0, 4, (16, 16, 16, 16), order=0)
    assert np.all(nearest == e0[1, 1])
    bilinear = pyramid.sample_e0_grid(e0, 1, (4, 4, 4, 4))
    assert bilinear.mean() == pytest.approx(e0[2:4, 2:4].mean())


def test_tile_path_pattern() -> None:
    """Tiles live in ``pyramid/E{level}/{col}_{row}.png`` like the manifest says."""
    path = pyramid.tile_path(Path("m"), pyramid.TileKey(3, 5, 7))
    assert path == Path("m/pyramid/E3/5_7.png")


# ----------------------------------------------------------------------------- RLE


def test_rle_round_trip() -> None:
    """Runs of consecutive columns per row, sorted, and back."""
    tiles = {(3, 1), (4, 1), (5, 1), (9, 1), (0, 4), (2, 4), (3, 4)}
    rle = pyramid.tiles_to_rle(tiles)
    assert rle == [
        {"row": 1, "runs": [[3, 3], [9, 1]]},
        {"row": 4, "runs": [[0, 1], [2, 2]]},
    ]
    assert pyramid.rle_to_tiles(rle) == tiles
    assert pyramid.tiles_to_rle([]) == []


# ------------------------------------------------------------------------ manifest

MANIFEST_SAMPLE = """{
  "description": "Manifeste.",
  "version": 1,
  "tile_px": 512,
  "root_tile_units": 256,
  "height_min_m": -200.0,
  "height_range_m": 5000.0,
  "dir": "pyramid",
  "pattern": "E{level}/{col}_{row}.png",
  "levels": [
    { "level": 1, "meters_per_px": 179.744, "tier": 1, "source": "GLO-90", "tiles_rle": [] },
    { "level": 2, "meters_per_px": 89.872, "tier": 1, "source": "GLO-90", "tiles_rle": [] },
    { "level": 5, "meters_per_px": 11.234, "tier": 3, "source": "MNT", "tiles_rle": [{ "row": 2, "runs": [[1, 2]] }] }
  ]
}
"""


def test_manifest_update_touches_only_its_lines(tmp_path: Path) -> None:
    """Rewriting E2 leaves every other line byte-identical (ZG3 writes E5-E7)."""
    path = tmp_path / "relief_pyramid.json"
    path.write_text(MANIFEST_SAMPLE, encoding="utf-8")
    before = MANIFEST_SAMPLE.split("\n")
    entry = json.loads(before[11].strip().rstrip(","))
    entry["tiles_rle"] = pyramid.tiles_to_rle({(4, 7), (5, 7)})
    pyramid.update_manifest_levels(path, {2: entry})
    after = path.read_text(encoding="utf-8").split("\n")
    changed = [i for i, (a, b) in enumerate(zip(before, after, strict=True)) if a != b]
    assert changed == [11]
    assert after[11].startswith("    { ") and after[11].endswith(" },")
    parsed = json.loads(path.read_text(encoding="utf-8"))
    assert parsed["levels"][1]["tiles_rle"] == [{"row": 7, "runs": [[4, 2]]}]


def test_manifest_inserts_missing_level_and_cache_line(tmp_path: Path) -> None:
    """A missing level is inserted in order; ``cache`` is one line, then replaced."""
    path = tmp_path / "relief_pyramid.json"
    path.write_text(MANIFEST_SAMPLE, encoding="utf-8")
    new = {
        "level": 3,
        "meters_per_px": 44.936,
        "tier": 2,
        "source": "GLO-30",
        "tiles_rle": [],
    }
    cache = {"generated_at": "x", "total_bytes": 1, "tile_count": 1}
    pyramid.update_manifest_levels(path, {3: new}, cache)
    text = path.read_text(encoding="utf-8")
    parsed = json.loads(text)
    assert [level["level"] for level in parsed["levels"]] == [1, 2, 3, 5]
    assert parsed["cache"] == cache
    # Untouched entries keep their text.
    for line in MANIFEST_SAMPLE.split("\n")[10:13]:
        assert line.rstrip(",") in text
    cache2 = {"generated_at": "y", "total_bytes": 2, "tile_count": 2}
    pyramid.update_manifest_levels(path, {}, cache2)
    lines = path.read_text(encoding="utf-8").split("\n")
    assert sum(line.startswith('  "cache":') for line in lines) == 1
    assert json.loads("\n".join(lines))["cache"] == cache2


def test_committed_manifest_keeps_one_line_per_level() -> None:
    """The versioned manifest follows the one-line-per-level contract."""
    lines = (MAP_DIR / "relief_pyramid.json").read_text(encoding="utf-8").split("\n")
    start = lines.index('  "levels": [')
    end = next(i for i in range(start, len(lines)) if lines[i].strip() in ("]", "],"))
    levels = [json.loads(line.strip().rstrip(",")) for line in lines[start + 1 : end]]
    assert [level["level"] for level in levels] == list(range(1, 8))


# -------------------------------------------------------------- boost and averaging


def test_boost_commutes_with_the_mean_below_the_clamp() -> None:
    """Without clamping nor valley floor, mean of boosted children = boosted parent."""
    rng = np.random.default_rng(3)
    children = rng.uniform(185.0, 260.0, (64, 64)).astype(np.float32)
    base = np.full_like(children, 180.0)
    boosted_mean = relief_shade.block_mean(pyramid.boost_with_base(children, base), 2)
    mean_boosted = pyramid.boost_with_base(
        relief_shade.block_mean(children, 2), relief_shade.block_mean(base, 2)
    )
    np.testing.assert_allclose(boosted_mean, mean_boosted, atol=1e-3)


def test_boost_matches_relief_shade_formula() -> None:
    """Same boost as ``heightmap_render.png`` (ADR 0019) given the same base."""
    rng = np.random.default_rng(5)
    heights = np.abs(rng.normal(300.0, 200.0, (96, 96))).astype(np.float32) + 1
    land = np.ones(heights.shape, dtype=bool)
    reference = relief_shade.boost_relief(heights, land, 359.488)
    base = pyramid.boost_base(heights, 359.488)
    np.testing.assert_allclose(
        pyramid.boost_with_base(heights, base), reference, atol=1e-3
    )


def test_apply_coast_and_fade() -> None:
    """Land >= 0.5 m, water <= 0, and a ramp over the pixels next to the shore."""
    heights = np.full((8, 8), 40.0, dtype=np.float32)
    water = np.zeros((8, 8), dtype=bool)
    water[:, :3] = True
    sea = np.full((8, 8), -20.0, dtype=np.float32)
    plain = pyramid.apply_coast(heights, water, sea)
    assert np.all(plain[:, :3] == -20.0) and np.all(plain[:, 3:] == 40.0)
    faded = pyramid.apply_coast(heights, water, sea, fade_px=2)
    row = faded[0]
    assert row[2] > row[1] > row[0] == -20.0
    assert relief_shade.MIN_LAND_M < row[3] < row[4] < row[5] == 40.0


# --------------------------------------------------------------- surface correction


def _plane(size: int = 160) -> np.ndarray:
    yy, xx = np.mgrid[:size, :size].astype(np.float32)
    return 100.0 + 0.01 * 22.5 * xx + 0.005 * 22.5 * yy


def test_canopy_is_removed_under_trees() -> None:
    """A forest raising the surface by the canopy offset goes back to the ground."""
    ground = _plane()
    trees = np.zeros_like(ground)
    trees[40:120, 40:120] = 1.0
    dsm = ground + surface.CANOPY_OFFSET_M * trees
    corrected = surface.remove_canopy(dsm, trees)
    np.testing.assert_allclose(
        corrected[50:110, 50:110], ground[50:110, 50:110], atol=0.05
    )
    np.testing.assert_allclose(corrected[:20, :20], ground[:20, :20], atol=0.05)


def test_edge_steps_measure_the_canopy() -> None:
    """The forest-edge statistic recovers a synthetic canopy offset on flat land."""
    ground = np.full((200, 200), 150.0, dtype=np.float32)
    trees = np.zeros_like(ground)
    trees[:, 100:] = 1.0
    steps = surface.canopy_edge_steps(ground + 12.0 * trees, trees, 22.5)
    assert steps.size > 0
    assert np.median(steps) == pytest.approx(12.0, abs=0.5)


def test_buildings_are_flattened() -> None:
    """Blocks of 15 m buildings on a slope give back the slope (within a metre)."""
    ground = _plane()
    built = np.zeros_like(ground)
    built[20:140, 20:140] = 1.0
    dsm = ground.copy()
    for r in range(24, 136, 8):
        for c in range(24, 136, 8):
            dsm[r : r + 5, c : c + 5] += 15.0
    flat = surface.flatten_built(dsm, dsm, built, 22.5)
    inner = flat[40:120, 40:120] - ground[40:120, 40:120]
    assert np.abs(inner).max() < 1.5
    assert np.all(flat[:10, :10] == dsm[:10, :10])


def test_push_pull_fills_a_hole_smoothly() -> None:
    """A hole in a plane is filled close to the plane."""
    plane = _plane(128)
    known = np.ones(plane.shape, dtype=bool)
    known[40:90, 30:100] = False
    filled = surface.push_pull(np.where(known, plane, 0.0), known)
    assert np.all(filled[known] == plane[known])
    assert np.abs(filled - plane).max() < 4.0


def test_reservoir_valley_is_reconstructed() -> None:
    """A flat lake in a V valley is replaced by a valley below the lake level."""
    size = 200
    yy, xx = np.mgrid[:size, :size].astype(np.float32)
    valley = 300.0 + 0.2 * 22.5 * np.abs(xx - 100)
    level = 360.0
    lake = valley < level
    lake[:, :] &= (yy > 40) & (yy < 160)
    dsm = np.where(lake, level, valley).astype(np.float32)
    water = lake.astype(np.float32)
    mask, fill, report = surface.solve_reservoir(
        dsm, dsm, water, (100, 100), 22.5, 80.0, 90
    )
    assert report["found"]
    assert report["level_m"] == pytest.approx(level)
    assert mask[100, 100] and not mask[100, 20]
    assert fill[100, 100] < level - 20.0
    assert np.all(fill[mask] <= level + 1e-3)


# ------------------------------------------------------------------------- data


def test_reservoirs_match_schema_and_core() -> None:
    """``modern_reservoirs.json`` is valid, post-1340 and inside the core bbox."""
    schema = json.loads(
        (DATA / "schemas" / "modern_reservoirs.schema.json").read_text(encoding="utf-8")
    )
    document = json.loads(
        (MAP_DIR / "modern_reservoirs.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(document))
    assert not errors, [error.message for error in errors]
    ids = [entry["id"] for entry in document["reservoirs"]]
    assert len(ids) == len(set(ids))
    lon_min, lat_min, lon_max, lat_max = pyramid.CORE_BBOX
    for entry in document["reservoirs"]:
        assert lon_min <= entry["lon"] <= lon_max, entry["id"]
        assert lat_min <= entry["lat"] <= lat_max, entry["id"]
        assert entry["dam_year"] > 1340


# ------------------------------------------------------------- baked cache (optional)


def _cached(level: int) -> list[tuple[int, int]]:
    return sorted(pyramid.scan_level(MAP_DIR, level))


@pytest.mark.skipif(not _cached(1), reason="pyramid cache not baked")
def test_e1_mean_matches_e0() -> None:
    """The 2 x 2 mean of E1 gives E0 back up to the processing differences.

    E0 averaged each 1° GLO-90 tile separately (seams up to ~300 m along integer
    meridians and parallels in the Alps); E1 mosaics the tiles first. Inland, the
    median gap stays under a metre and the mean is unbiased.
    """
    diffs = []
    for col, row in _cached(1)[::7]:
        key = pyramid.TileKey(1, col, row)
        child = terrain.uint16_to_height(
            terrain.read_png16(pyramid.tile_path(MAP_DIR, key))
        )
        e0_path = MAP_DIR / "height" / f"h_{col // 2}_{row // 2}.png"
        parent = terrain.uint16_to_height(terrain.read_png16(e0_path))
        dy, dx = row % 2, col % 2
        parent = parent[dy * 256 : (dy + 1) * 256, dx * 256 : (dx + 1) * 256]
        mean = relief_shade.block_mean(child.astype(np.float32), 2)
        land = parent > 5.0
        diffs.append((mean - parent)[land])
    gap = np.concatenate(diffs)
    assert abs(float(gap.mean())) < 0.5
    assert float(np.median(np.abs(gap))) < 1.0


@pytest.mark.skipif(not _cached(2), reason="pyramid cache not baked")
def test_e2_mean_is_e1() -> None:
    """E1 is the 2 x 2 mean of E2 (baked together): equal to the quantization."""
    checked = 0
    for col, row in _cached(2)[::25]:
        key = pyramid.TileKey(2, col, row)
        parent_path = pyramid.tile_path(MAP_DIR, key.parent())
        if not parent_path.exists():
            continue
        child = terrain.uint16_to_height(
            terrain.read_png16(pyramid.tile_path(MAP_DIR, key))
        )
        parent = terrain.uint16_to_height(terrain.read_png16(parent_path))
        dy, dx = row % 2, col % 2
        parent = parent[dy * 256 : (dy + 1) * 256, dx * 256 : (dx + 1) * 256]
        mean = relief_shade.block_mean(child.astype(np.float32), 2)
        land = parent > 1.0
        assert np.percentile(np.abs(mean - parent)[land], 99) < 0.2
        checked += 1
    assert checked > 0


def test_tile_write_is_atomic(tmp_path: Path) -> None:
    """Tiles are written through a hidden partial file, never left half-written."""
    map_dir = tmp_path / "map"
    key = pyramid.TileKey(3, 1, 2)
    heights = np.linspace(-10, 900, 512 * 512, dtype=np.float32).reshape(512, 512)
    size = pyramid._write_tile(map_dir, key, heights)
    path = pyramid.tile_path(map_dir, key)
    assert size == path.stat().st_size
    assert [p.name for p in path.parent.iterdir()] == ["1_2.png"]
    decoded = terrain.uint16_to_height(terrain.read_png16(path))
    np.testing.assert_allclose(decoded, heights, atol=0.05)
    assert pyramid.scan_level(map_dir, 3) == {(1, 2)}
    shutil.rmtree(map_dir)


def test_pyramid_frame_sits_at_root_origin_tiles() -> None:
    """OM2 (ADR 0115): the baked pyramid keeps the former 4096² frame at (0, 5) root tiles."""
    manifest = json.loads((MAP_DIR / "relief_pyramid.json").read_text(encoding="utf-8"))
    assert manifest["root_origin_tiles"] == [0, 5]
    assert pyramid.root_origin_tiles(MAP_DIR) == (0, 5)
    assert pyramid.map_bounds(MAP_DIR) == (2169486.0, 1327858.0, 5114414.0, 4272786.0)
    with pytest.raises(RuntimeError):
        pyramid.require_world_frame(MAP_DIR, "test")


def test_e0_heights_read_the_frame_window(tmp_path: Path) -> None:
    """E0 of the frame is read from the world tiles shifted by root_origin_tiles."""
    (tmp_path / "relief_pyramid.json").write_text(
        json.dumps({"root_origin_tiles": [1, 2]}), encoding="utf-8"
    )
    height_dir = tmp_path / "height"
    height_dir.mkdir()
    for row in range(2, 2 + pyramid.E0_TILES):
        for col in range(1, 1 + pyramid.E0_TILES):
            value = terrain.height_to_uint16(float(col * 100 + row))
            terrain.write_png16(
                np.full((pyramid.TILE_PX, pyramid.TILE_PX), value, dtype=np.uint16),
                height_dir / f"h_{col}_{row}.png",
            )
    e0 = pyramid.e0_heights(tmp_path)
    assert e0.shape == (8192, 8192)
    assert abs(float(e0[0, 0]) - 102.0) < 0.1  # world tile (1, 2)
    assert abs(float(e0[-1, -1]) - (16 * 100 + 17)) < 0.1  # world tile (16, 17)
