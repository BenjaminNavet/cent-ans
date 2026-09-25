"""GLO-30 surface model -> terrain model, tier 2 of the relief pyramid (lot ZG1).

Copernicus GLO-30 is a *digital surface model*: X-band radar sees the top of
forests and buildings, and the water surface of reservoirs. FABDEM (which removes
them) is non-commercial, so ADR 0036 corrects GLO-30 here:

- **Canopy**: under ESA WorldCover tree cover, :data:`CANOPY_OFFSET_M` metres
  times the (lightly blurred) tree fraction are removed. The offset is the step
  measured across forest edges on flat ground (see :func:`canopy_edge_steps`),
  i.e. the part of the canopy the radar actually sees, not the tree height.
- **Modern built-up** (WorldCover class 50): a morphological ground estimate
  (grey opening of :data:`BUILT_OPENING_M`, then a light blur) replaces the
  surface, feathered at the edge of the built mask. It is local, so tier-2
  blocks stay seamless, and it keeps the valleys and hills of large cities
  (Paris spans 40 km: an interpolation from its edges would erase the Seine).
- **Post-1340 reservoirs** of ``data/map/modern_reservoirs.json``: the flat water
  surface (WorldCover water, level of the seed point) is replaced by a membrane
  interpolated from the banks (push-pull) minus a valley profile that continues
  the bank slopes, capped by the dam height. Each reservoir is solved once on
  its own window (:func:`prepare_reservoirs`) and pasted into every block.

The E4 block (a 2048² E2 footprint plus :data:`pyramid.TIER2_MARGIN_PX`) is then
boosted like every level, its coast set (source coast near E0's shore, E0's
water elsewhere, 2-pixel fade) and split into E4 tiles; E3 is its 2 x 2 mean.
"""

from __future__ import annotations

import json
import math
from pathlib import Path

import numpy as np
import rasterio
from rasterio.enums import Resampling
from rasterio.merge import merge
from scipy import ndimage

from cent_ans_tools.geo import glo30, pyramid, relief_shade
from cent_ans_tools.geo.project import MapGrid

RESERVOIRS_FILE = "modern_reservoirs.json"
RESERVOIR_DIR = pyramid.WORK_DIR / "reservoirs"

#: Metres of DSM removed under full tree cover (measured, see module docstring).
CANOPY_OFFSET_M = 10.0
#: Blur (pixels of E4) of the tree fraction before the canopy is removed.
CANOPY_BLUR_PX = 1.5
#: Width of the grey opening that removes buildings (larger than a city block).
BUILT_OPENING_M = 340.0
#: Width of the closing that fills radar pits left between buildings.
BUILT_PIT_M = 180.0
#: Built-up fraction above which the ground estimate replaces the surface.
BUILT_THRESHOLD = 0.3
#: WorldCover read at 1/2 resolution (≈ 20 m, internal overviews): enough for 22 m.
WORLDCOVER_DECIMATION = 2
GLO30_RES_DEG = 1.0 / 3600.0
WORLDCOVER_RES_DEG = WORLDCOVER_DECIMATION / 12000.0
#: Reservoir water: WorldCover water fraction and tolerance around the lake level.
RESERVOIR_WATER_FRACTION = 0.5
RESERVOIR_LEVEL_TOLERANCE_M = 1.5
RESERVOIR_SEED_RADIUS_M = 3000.0
RESERVOIR_DEPTH_OF_DAM = 0.8
RESERVOIR_SLOPE_RANGE = (0.02, 0.35)


# ------------------------------------------------------------------------ sources


def prepare_sources(bbox: tuple[float, float, float, float]) -> None:
    """Make sure the GLO-30 and WorldCover tiles of ``bbox`` are cached."""
    glo30.fetch_worldcover(glo30.worldcover_tiles_in_bbox(bbox))
    glo30.fetch_glo30(glo30.glo30_tiles_in_bbox(glo30.glo30_tile_list(), bbox))


def _glo30_paths(bbox: tuple[float, float, float, float]) -> list[Path]:
    """Cached GLO-30 tiles intersecting ``bbox`` (sea cells have no tile)."""
    lon_min, lat_min, lon_max, lat_max = bbox
    paths = []
    for lat in range(math.floor(lat_min), math.ceil(lat_max)):
        for lon in range(math.floor(lon_min), math.ceil(lon_max)):
            path = glo30.glo30_path(glo30.glo30_tile_name(lon, lat))
            if path.exists():
                paths.append(path)
    return paths


def _worldcover_paths(bbox: tuple[float, float, float, float]) -> list[Path]:
    """Cached WorldCover tiles intersecting ``bbox``."""
    return [
        path
        for path in map(glo30.worldcover_path, glo30.worldcover_tiles_in_bbox(bbox))
        if path.exists()
    ]


def read_dsm(grid: MapGrid, window: tuple[int, int, int, int]) -> np.ndarray:
    """GLO-30 heights (bilinear) on a grid window; NaN where no tile (open sea)."""
    paths = _glo30_paths(glo30.window_lonlat(grid, window))
    return glo30.mosaic_to_grid(paths, grid, window, GLO30_RES_DEG, Resampling.bilinear)


def read_cover(
    grid: MapGrid, window: tuple[int, int, int, int]
) -> dict[str, np.ndarray]:
    """WorldCover fractions (``trees``, ``built``, ``water``) on a grid window."""
    shape = (window[3], window[2])
    bbox = glo30.window_lonlat(grid, window)
    paths = _worldcover_paths(bbox)
    empty = {
        name: np.zeros(shape, dtype=np.float32) for name in ("trees", "built", "water")
    }
    if not paths:
        return empty
    datasets = [rasterio.open(path) for path in paths]
    try:
        classes, transform = merge(
            datasets,
            bounds=bbox,
            res=(WORLDCOVER_RES_DEG, WORLDCOVER_RES_DEG),
            nodata=0,
            resampling=Resampling.nearest,
        )
    finally:
        for dataset in datasets:
            dataset.close()
    classes = classes[0]
    known = classes > 0
    names = ("trees", "built", "water")
    values = (glo30.WC_TREES, glo30.WC_BUILT, glo30.WC_WATER)
    fractions = np.stack(
        [np.where(known, (classes == v).astype(np.float32), np.nan) for v in values]
    )
    warped = np.nan_to_num(
        glo30.warp_to_grid(fractions, transform, grid, window, Resampling.average),
        nan=0.0,
    )
    return dict(zip(names, warped, strict=True))


# --------------------------------------------------------------------- correction


def remove_canopy(
    dsm_m: np.ndarray, trees: np.ndarray, offset_m: float = CANOPY_OFFSET_M
) -> np.ndarray:
    """Lower the surface by ``offset_m`` times the blurred tree fraction."""
    cover = ndimage.gaussian_filter(trees.astype(np.float32), CANOPY_BLUR_PX)
    return (dsm_m - offset_m * np.clip(cover, 0.0, 1.0)).astype(np.float32)


def flatten_built(
    dsm_m: np.ndarray,
    terrain_m: np.ndarray,
    built: np.ndarray,
    meters_per_px: float,
) -> np.ndarray:
    """Replace built-up areas by a morphological ground estimate (feathered).

    The ground is the grey opening of the *raw* surface by a disc of
    :data:`BUILT_OPENING_M` (streets, squares and yards are ground; blocks and
    buildings narrower than the disc go), computed at half resolution on the
    2 x 2 minimum, then smoothed. The raw surface is used because the canopy
    correction digs pits under street trees that an opening would spread into
    square terraces.
    """
    mask = ndimage.gaussian_filter(built.astype(np.float32), 1.0) > BUILT_THRESHOLD
    if not mask.any():
        return terrain_m
    rows, cols = dsm_m.shape
    padded = np.pad(dsm_m, ((0, rows % 2), (0, cols % 2)), mode="edge")
    half = padded.reshape(padded.shape[0] // 2, 2, padded.shape[1] // 2, 2).min(
        axis=(1, 3)
    )
    disc = _disc(BUILT_OPENING_M / (4.0 * meters_per_px))
    opened = ndimage.grey_opening(half, footprint=disc)
    # Radar shadows between buildings leave pits: a small closing fills them.
    opened = ndimage.grey_closing(
        opened, footprint=_disc(BUILT_PIT_M / (4.0 * meters_per_px))
    )
    ground = _upsample_to(opened, padded.shape, (2.0, 2.0))[:rows, :cols]
    ground = ndimage.gaussian_filter(ground.astype(np.float32), 2.0)
    ground = np.minimum(ground, dsm_m)
    weight = np.clip(
        ndimage.gaussian_filter(
            ndimage.binary_dilation(mask, iterations=1).astype(np.float32), 1.5
        )
        * 1.5,
        0.0,
        1.0,
    )
    return (terrain_m * (1.0 - weight) + ground * weight).astype(np.float32)


def _disc(radius_px: float) -> np.ndarray:
    """Boolean disc footprint of the given radius (at least one pixel)."""
    radius = max(1, int(round(radius_px)))
    yy, xx = np.ogrid[-radius : radius + 1, -radius : radius + 1]
    return xx * xx + yy * yy <= radius * radius


def correct_surface(
    dsm_m: np.ndarray, cover: dict[str, np.ndarray], meters_per_px: float
) -> np.ndarray:
    """Terrain estimate from the surface model (canopy, then built-up)."""
    filled = np.nan_to_num(dsm_m, nan=0.0)
    terrain_m = remove_canopy(filled, cover["trees"])
    terrain_m = flatten_built(filled, terrain_m, cover["built"], meters_per_px)
    # Water and sea keep the source value (canopy blur must not dig lakes).
    water = (cover["water"] > 0.5) | (np.abs(filled) <= 0.01)
    return np.where(water, filled, terrain_m).astype(np.float32)


def push_pull(values: np.ndarray, known: np.ndarray) -> np.ndarray:
    """Smooth interpolation of the unknown pixels from the known ones.

    Pull: weighted 2 x 2 sums down to one pixel; push: each level fills its
    empty pixels with the bilinear upsampling of the coarser estimate. Linear in
    the number of pixels; the result is a membrane-like surface anchored on the
    known pixels (kept as they are).
    """
    values = values.astype(np.float64)
    weight = known.astype(np.float64)
    stack = []
    level_v, level_w = values * weight, weight
    while min(level_v.shape) > 1:
        stack.append((level_v, level_w))
        rows, cols = level_v.shape
        pad = ((0, rows % 2), (0, cols % 2))
        v = np.pad(level_v, pad)
        w = np.pad(level_w, pad)
        level_v = v.reshape(v.shape[0] // 2, 2, v.shape[1] // 2, 2).sum(axis=(1, 3))
        level_w = w.reshape(w.shape[0] // 2, 2, w.shape[1] // 2, 2).sum(axis=(1, 3))
    estimate = level_v / np.maximum(level_w, 1e-12)
    for v, w in reversed(stack):
        zoom = (v.shape[0] / estimate.shape[0], v.shape[1] / estimate.shape[1])
        up = _upsample_to(estimate, v.shape, zoom)
        local = v / np.maximum(w, 1e-12)
        alpha = np.clip(w, 0.0, 1.0)
        estimate = alpha * local + (1.0 - alpha) * up
    return np.where(known, values, estimate).astype(np.float32)


def _upsample_to(array: np.ndarray, shape: tuple[int, int], zoom: tuple) -> np.ndarray:
    """Bilinear upsampling of ``array`` to ``shape`` (pixel centres aligned)."""
    rows = (np.arange(shape[0]) + 0.5) / zoom[0] - 0.5
    cols = (np.arange(shape[1]) + 0.5) / zoom[1] - 0.5
    return pyramid._interp_axis(
        pyramid._interp_axis(array.astype(np.float32), rows, 0), cols, 1
    ).astype(np.float64)


def canopy_edge_steps(
    dsm_m: np.ndarray, trees: np.ndarray, meters_per_px: float
) -> np.ndarray:
    """Surface steps (m) between forest interiors and open ground on flat land.

    Forest interior = tree fraction > 0.8 at least 4 pixels (≈ 90 m) from open
    ground; open ground = tree fraction < 0.05 as far from the forest. Around
    each pixel where both are present within ≈ 225 m and the open ground is flat
    (slope < 2 %), the mean surface of the forest interior minus that of the open
    ground: what the radar sees above the ground under a closed canopy.
    """
    forest = trees > 0.8
    open_ground = trees < 0.05
    interior = ndimage.distance_transform_edt(~open_ground) >= 4
    interior &= forest
    far_open = ndimage.distance_transform_edt(~forest) >= 4
    far_open &= open_ground
    sigma = 10.0
    w_in = ndimage.gaussian_filter(interior.astype(np.float32), sigma)
    w_out = ndimage.gaussian_filter(far_open.astype(np.float32), sigma)
    h_in = ndimage.gaussian_filter(dsm_m * interior, sigma) / np.maximum(w_in, 1e-6)
    h_out = ndimage.gaussian_filter(dsm_m * far_open, sigma) / np.maximum(w_out, 1e-6)
    gy, gx = np.gradient(h_out, meters_per_px)
    flat = np.hypot(gx, gy) < 0.02
    sample = (w_in > 0.15) & (w_out > 0.15) & flat
    # One sample every 8 pixels: neighbouring values are strongly correlated.
    sample[::8, ::8] &= True
    thin = np.zeros_like(sample)
    thin[::8, ::8] = sample[::8, ::8]
    return (h_in - h_out)[thin]


# --------------------------------------------------------------------- reservoirs


def load_reservoirs(map_dir: Path) -> list[dict]:
    """Entries of ``data/map/modern_reservoirs.json``."""
    path = map_dir / RESERVOIRS_FILE
    if not path.exists():
        return []
    return json.loads(path.read_text(encoding="utf-8"))["reservoirs"]


def reservoir_window(grid: MapGrid, entry: dict) -> tuple[int, int, int, int]:
    """E4 window around a reservoir: its search box plus a 3 km margin."""
    px, py = grid.lonlat_to_pixel(entry["lon"], entry["lat"])
    half = int(math.ceil((entry["search_km"] * 1000.0 + 3000.0) / grid.meters_per_px))
    return int(px) - half, int(py) - half, 2 * half, 2 * half


def solve_reservoir(
    dsm_m: np.ndarray,
    terrain_m: np.ndarray,
    water_fraction: np.ndarray,
    seed_rc: tuple[int, int],
    meters_per_px: float,
    dam_height_m: float,
    search_px: int,
) -> tuple[np.ndarray, np.ndarray, dict]:
    """Mask and pre-dam valley of one reservoir on its window.

    Args:
        dsm_m: Source surface (flat water at the lake level).
        terrain_m: Corrected terrain (banks).
        water_fraction: WorldCover water fraction.
        seed_rc: ``(row, col)`` of the listed point.
        meters_per_px: Pixel size.
        dam_height_m: Height of the dam (caps the reconstructed depth).
        search_px: Half-side of the search box around the seed.

    Returns:
        ``(mask, fill, report)``; ``mask`` is empty when no lake was found.
    """
    rows, cols = dsm_m.shape
    seed_r, seed_c = seed_rc
    box = np.zeros(dsm_m.shape, dtype=bool)
    box[
        max(seed_r - search_px, 0) : min(seed_r + search_px, rows),
        max(seed_c - search_px, 0) : min(seed_c + search_px, cols),
    ] = True
    water = (water_fraction > RESERVOIR_WATER_FRACTION) & box
    report: dict = {"found": False}
    if not water.any():
        return np.zeros_like(water), terrain_m, report
    # Largest water component within reach of the seed (a nearer pond, a river
    # or a glacier lake must not win).
    labels, _ = ndimage.label(water)
    radius = int(RESERVOIR_SEED_RADIUS_M / meters_per_px)
    yy, xx = np.ogrid[:rows, :cols]
    reach = (yy - seed_r) ** 2 + (xx - seed_c) ** 2 <= radius**2
    near = labels[reach & water]
    if near.size == 0:
        return np.zeros_like(water), terrain_m, report
    seed_label = labels[seed_r, seed_c]
    component = labels == (seed_label or np.bincount(near).argmax())
    # Copernicus flattens lakes to one value: the most frequent height of the
    # component is the lake level (a median would mix in the river below the dam).
    heights = dsm_m[component]
    values, counts = np.unique(np.round(heights * 2.0) / 2.0, return_counts=True)
    level = float(values[np.argmax(counts)])
    level = float(np.median(heights[np.abs(heights - level) < 0.5]))
    # WorldCover 2021 and the DSM (2011-2015) saw different lake levels: the lake
    # is the WorldCover water down to a little below the level (not the river
    # below the dam) joined to the strictly flat DSM surface at the level.
    kept = component & (dsm_m >= level - RESERVOIR_LEVEL_TOLERANCE_M)
    flat = box & (np.abs(dsm_m - level) < 0.3)
    labels, _ = ndimage.label(flat | kept)
    candidates = labels[kept]
    candidates = candidates[candidates > 0]
    if candidates.size == 0:
        return np.zeros_like(water), terrain_m, report
    lake = labels == np.bincount(candidates).argmax()
    # The radar shore and the bilinear resampling blur the edge: two pixels more.
    mask = ndimage.binary_dilation(lake, iterations=2) & box
    membrane = push_pull(terrain_m, ~mask)
    ring = ndimage.binary_dilation(mask, iterations=8) & ~mask
    smooth = ndimage.gaussian_filter(terrain_m, 1.5)
    gy, gx = np.gradient(smooth, meters_per_px)
    slope = float(np.clip(np.median(np.hypot(gx, gy)[ring]), *RESERVOIR_SLOPE_RANGE))
    to_bank = ndimage.distance_transform_edt(mask) * meters_per_px
    cap = RESERVOIR_DEPTH_OF_DAM * dam_height_m
    depth = np.minimum(slope * to_bank, cap)
    depth = ndimage.gaussian_filter(np.where(mask, depth, 0.0), 2.0)
    fill = np.minimum(membrane - depth, level).astype(np.float32)
    report = {
        "found": True,
        "level_m": round(level, 1),
        "area_km2": round(float(lake.sum()) * meters_per_px**2 / 1e6, 2),
        "bank_slope": round(slope, 3),
        "max_depth_m": round(float(depth[mask].max()), 1),
    }
    return mask, fill, report


def prepare_reservoirs(map_dir: Path, force: bool = False) -> dict[str, Path]:
    """Solve every listed reservoir once (cached ``.npz``), write a report."""
    RESERVOIR_DIR.mkdir(parents=True, exist_ok=True)
    grid = pyramid.level_grid(pyramid.map_bounds(map_dir), 4)
    index, reports = [], {}
    for entry in load_reservoirs(map_dir):
        path = RESERVOIR_DIR / f"{entry['id']}.npz"
        window = reservoir_window(grid, entry)
        if force or not path.exists():
            dsm = read_dsm(grid, window)
            cover = read_cover(grid, window)
            terrain_m = correct_surface(dsm, cover, grid.meters_per_px)
            half = window[2] // 2
            mask, fill, report = solve_reservoir(
                np.nan_to_num(dsm),
                terrain_m,
                cover["water"],
                (half, half),
                grid.meters_per_px,
                entry["dam_height_m"],
                int(entry["search_km"] * 1000.0 / grid.meters_per_px),
            )
            report["expected_area_km2"] = entry.get("area_km2")
            np.savez_compressed(
                path,
                window=np.array(window),
                mask=mask,
                fill=np.where(mask, fill, 0.0).astype(np.float32),
                report=json.dumps(report),
            )
        with np.load(path) as data:
            reports[entry["id"]] = json.loads(str(data["report"]))
        if reports[entry["id"]].get("found"):
            index.append({"id": entry["id"], "window": list(window)})
    (RESERVOIR_DIR / "index.json").write_text(json.dumps(index), encoding="utf-8")
    (RESERVOIR_DIR / "report.json").write_text(
        json.dumps(reports, indent=1, ensure_ascii=False), encoding="utf-8"
    )
    return {}


def apply_reservoirs(
    terrain_m: np.ndarray, window: tuple[int, int, int, int]
) -> np.ndarray:
    """Paste the solved reservoir valleys intersecting ``window``."""
    index_path = RESERVOIR_DIR / "index.json"
    if not index_path.exists():
        return terrain_m
    col0, row0, cols, rows = window
    result = terrain_m
    for item in json.loads(index_path.read_text(encoding="utf-8")):
        r_col0, r_row0, r_cols, r_rows = item["window"]
        c_lo, c_hi = max(col0, r_col0), min(col0 + cols, r_col0 + r_cols)
        r_lo, r_hi = max(row0, r_row0), min(row0 + rows, r_row0 + r_rows)
        if c_lo >= c_hi or r_lo >= r_hi:
            continue
        with np.load(RESERVOIR_DIR / f"{item['id']}.npz") as data:
            mask = data["mask"][
                r_lo - r_row0 : r_hi - r_row0, c_lo - r_col0 : c_hi - r_col0
            ]
            fill = data["fill"][
                r_lo - r_row0 : r_hi - r_row0, c_lo - r_col0 : c_hi - r_col0
            ]
        if result is terrain_m:
            result = terrain_m.copy()
        target = result[r_lo - row0 : r_hi - row0, c_lo - col0 : c_hi - col0]
        target[mask] = fill[mask]
    return result


# ------------------------------------------------------------------------- tier 2


def tier2_heights(
    grid: MapGrid,
    window: tuple[int, int, int, int],
    e0: np.ndarray,
    base: np.ndarray,
    coast: np.ndarray,
    map_dir: Path = pyramid.MAP_DIR,
) -> tuple[np.ndarray, np.ndarray]:
    """Boosted E4 heights and water mask of a window (margin included)."""
    dsm = read_dsm(grid, window)
    cover = read_cover(grid, window)
    terrain_m = correct_surface(dsm, cover, grid.meters_per_px)
    terrain_m = apply_reservoirs(terrain_m, window)
    missing = np.isnan(dsm)
    source_sea = ~missing & (np.abs(np.nan_to_num(dsm)) <= 0.01)
    e0_bil = pyramid.sample_e0_grid(e0, 4, window)
    coast_class = pyramid.sample_e0_grid(coast, 4, window, order=0)
    # Near E0's shore the source decides; elsewhere (and without source) E0 does.
    water = (coast_class == 0) | (
        (coast_class == 1) & (source_sea | (missing & (e0_bil <= 0.0)))
    )
    base_bil = pyramid.sample_e0_grid(base, 4, window)
    heights = pyramid.boost_with_base(terrain_m, base_bil)
    no_source = missing | source_sea
    if no_source.any():
        # Land without GLO-30 (edge of the core, source sea inside E0's land):
        # the E2 tier-1 surface, already boosted, or E0 without it.
        fallback = pyramid.sample_level_tiles(map_dir, 2, 4, window)
        fallback = np.where(np.isnan(fallback), e0_bil, fallback)
        heights = np.where(no_source, fallback, heights)
    heights = pyramid.apply_coast(heights, water, e0_bil, pyramid.COAST_FADE_PX)
    return heights, water


def tier2_unit(
    key: pyramid.TileKey,
    write_levels: tuple[int, ...],
    e3_tiles: list,
    e4_tiles: list,
) -> list[tuple[int, int]]:
    """Bake the E3 and E4 tiles of one E2 footprint.

    Returns:
        ``(level, bytes)`` of every tile written.
    """
    shared = pyramid._SHARED
    map_dir: Path = shared["map_dir"]  # type: ignore[assignment]
    grid = pyramid.level_grid(shared["bounds"], 4)  # type: ignore[arg-type]
    size = 4 * pyramid.TILE_PX
    margin = pyramid.TIER2_MARGIN_PX
    col0, row0 = key.col * size, key.row * size
    window = (col0 - margin, row0 - margin, size + 2 * margin, size + 2 * margin)
    heights, water = tier2_heights(
        grid,
        window,
        shared["e0"],
        shared["base"],
        shared["coast"],  # type: ignore[arg-type]
    )
    h4 = heights[margin : margin + size, margin : margin + size]
    water4 = water[margin : margin + size, margin : margin + size]
    written: list[tuple[int, int]] = []
    tile = pyramid.TILE_PX
    if 4 in write_levels:
        for col, row in e4_tiles:
            dy, dx = row - 4 * key.row, col - 4 * key.col
            block = h4[dy * tile : (dy + 1) * tile, dx * tile : (dx + 1) * tile]
            written.append(
                (4, pyramid._write_tile(map_dir, pyramid.TileKey(4, col, row), block))
            )
    if 3 in write_levels:
        h3 = relief_shade.block_mean(h4, 2)
        water3 = relief_shade.block_mean(water4.astype(np.float32), 2) > 0.5
        h3 = pyramid.apply_coast(h3, water3, h3)
        for col, row in e3_tiles:
            dy, dx = row - 2 * key.row, col - 2 * key.col
            block = h3[dy * tile : (dy + 1) * tile, dx * tile : (dx + 1) * tile]
            written.append(
                (3, pyramid._write_tile(map_dir, pyramid.TileKey(3, col, row), block))
            )
    return written
