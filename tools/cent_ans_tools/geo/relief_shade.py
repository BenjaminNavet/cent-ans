"""Fine relief of the campaign map from Copernicus DEM GLO-90 (lot R1, ADR 0019).

Outputs in ``data/map/`` (same EPSG:3035 extent as ``heightmap.png``):

``height/h_<col>_<row>.png``
    The 8192² fine relief tiles (see :mod:`cent_ans_tools.geo.relief`), now the
    *area average* of Copernicus GLO-90 (≈ 90 m) instead of ETOPO bilinearly
    resampled (≈ 460 m): real detail down to one 8192 pixel (≈ 360 m). ETOPO is
    kept at sea (bathymetry) and outside :data:`copernicus.FINE_BBOX`.
``heightmap_render.png``
    4096² 16-bit, same encoding as ``heightmap.png``: the 2 x 2 mean of the fine
    tiles, i.e. the surface the renderer draws. Read by ``MapData`` instead of
    ``heightmap.png`` when present, so that terrain, armies, towns, rivers and the
    picker stay on one surface. ``heightmap.png`` (read by the navgrid builder) is
    left untouched: the game rules never see the render surface.
``relief_shade.png``
    8192² LA8: L = fine detail of the render surface relative to the bilinear
    4096² heightmap (``128 + m / DETAIL_SCALE_M``) for shading normals below the
    4096 pixel; A = multi-scale occlusion / curvature (128 neutral, darker in
    hollows and valley floors, lighter on crests).

Render-only relief enhancement (unsharp mask): on land, the local relief
``h - blur(h)`` (σ ≈ 5 km) is amplified by :data:`BOOST_GAIN` and limited to
±:data:`BOOST_LIMIT_M`, fading out above :data:`BOOST_FADE_M` so that rolling
plains read as hills without making the Alps absurd. The sign of every pixel
relative to sea level follows ``heightmap.png`` (coasts do not move).
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.geo import copernicus, download, relief, terrain
from cent_ans_tools.geo.project import MapGrid

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
CACHE_DIR = download.RAW_DIR / "copernicus_cache"
RENDER_HEIGHTMAP = "heightmap_render.png"
RELIEF_SHADE = "relief_shade.png"

BOOST_SIGMA_M = 5000.0
BOOST_GAIN = 0.8
BOOST_LIMIT_M = 120.0
BOOST_FADE_M = (600.0, 1600.0)
DETAIL_SCALE_M = 3.0  # metres per level of the detail channel (±381 m)
#: Curvature scales (metres) and normalisation (metres of local relief) of the occlusion channel.
OCCLUSION_SCALES = ((500.0, 12.0, 0.3), (1500.0, 35.0, 0.4), (4500.0, 90.0, 0.3))
MIN_LAND_M = 0.5


@dataclass(frozen=True)
class ReliefShadeResult:
    """Output of :func:`build`."""

    tiles: int
    tiles_bytes: int
    render_heightmap: Path
    relief_shade: Path
    shade_bytes: int
    seconds: float


def copernicus_on_grid(
    grid: MapGrid, names: list[str], block: int = 512, cache: Path | None = None
) -> np.ndarray:
    """Copernicus heights averaged on the whole ``grid`` (NaN where no tile), cached."""
    if cache is not None and cache.exists():
        return np.load(cache)
    size = grid.size_px
    result = np.full((size, size), np.nan, dtype=np.float32)
    lon_min, lat_min, lon_max, lat_max = copernicus.FINE_BBOX
    for row0 in range(0, size, block):
        for col0 in range(0, size, block):
            window = (col0, row0, block, block)
            w_lon0, w_lon1, w_lat0, w_lat1 = copernicus._window_lonlat(grid, window)
            if w_lon1 < lon_min or w_lon0 > lon_max or w_lat1 < lat_min or w_lat0 > lat_max:
                continue
            result[row0 : row0 + block, col0 : col0 + block] = (
                copernicus.resample_to_grid(grid, window, names)
            )
    if cache is not None:
        cache.parent.mkdir(parents=True, exist_ok=True)
        np.save(cache, result)
    return result


def upsample_sign(original_4096: np.ndarray, factor: int) -> np.ndarray:
    """Land flag of each fine pixel: the flag of its parent ``heightmap.png`` pixel."""
    land = original_4096 > 0.0
    return np.repeat(np.repeat(land, factor, axis=0), factor, axis=1)


def boost_relief(height_m: np.ndarray, land: np.ndarray, meters_per_px: float) -> np.ndarray:
    """Render-only unsharp mask of the local relief on land (see module docstring)."""
    sigma = BOOST_SIGMA_M / meters_per_px
    base = ndimage.gaussian_filter(height_m, sigma)
    local = np.clip(height_m - base, -BOOST_LIMIT_M, BOOST_LIMIT_M)
    t = np.clip((base - BOOST_FADE_M[0]) / (BOOST_FADE_M[1] - BOOST_FADE_M[0]), 0.0, 1.0)
    fade = 1.0 - t * t * (3.0 - 2.0 * t)
    boosted = height_m + BOOST_GAIN * local * fade
    return np.where(land, boosted, height_m).astype(np.float32)


def enforce_coast(height_m: np.ndarray, land: np.ndarray) -> np.ndarray:
    """Land pixels stay above :data:`MIN_LAND_M`, sea pixels at or below 0 m."""
    return np.where(land, np.maximum(height_m, MIN_LAND_M), np.minimum(height_m, 0.0)).astype(
        np.float32
    )


def block_mean(array: np.ndarray, factor: int) -> np.ndarray:
    """Mean of ``factor`` x ``factor`` blocks."""
    rows, cols = array.shape[0] // factor, array.shape[1] // factor
    return array.reshape(rows, factor, cols, factor).mean(axis=(1, 3)).astype(np.float32)


def bilinear_upsample(array: np.ndarray, factor: int) -> np.ndarray:
    """What a GPU bilinear fetch of ``array`` returns at the centres of a ``factor``x finer grid."""
    size = array.shape[0] * factor
    coords = (np.arange(size, dtype=np.float32) + 0.5) / factor - 0.5
    rows, cols = np.meshgrid(coords, coords, indexing="ij")
    return ndimage.map_coordinates(array, [rows, cols], order=1, mode="nearest").astype(
        np.float32
    )


def occlusion(height_m: np.ndarray, meters_per_px: float) -> np.ndarray:
    """Multi-scale curvature in about ``[-1, 1]``: negative in hollows, positive on crests."""
    total = np.zeros(height_m.shape, dtype=np.float32)
    for scale_m, norm_m, weight in OCCLUSION_SCALES:
        local = height_m - ndimage.gaussian_filter(height_m, scale_m / meters_per_px)
        total += weight * np.tanh(local / norm_m)
    return total


def encode_shade(detail_m: np.ndarray, occ: np.ndarray) -> np.ndarray:
    """``(rows, cols, 2)`` ``uint8``: detail (L) and occlusion (A), 128 = neutral."""
    detail = np.clip(np.rint(128.0 + detail_m / DETAIL_SCALE_M), 0, 255)
    occ8 = np.clip(np.rint(128.0 + 127.0 * occ), 0, 255)
    return np.stack([detail, occ8], axis=-1).astype(np.uint8)


def update_map_json(map_dir: Path, names: list[str]) -> None:
    """Record the render heightmap, the shade raster and the Copernicus source."""
    path = map_dir / "map.json"
    metadata = json.loads(path.read_text(encoding="utf-8"))
    metadata["render_heightmap"] = {
        "file": RENDER_HEIGHTMAP,
        "boost": {
            "sigma_m": BOOST_SIGMA_M,
            "gain": BOOST_GAIN,
            "limit_m": BOOST_LIMIT_M,
            "fade_m": list(BOOST_FADE_M),
        },
    }
    metadata["relief_shade"] = {"file": RELIEF_SHADE, "detail_scale_m": DETAIL_SCALE_M}
    metadata.setdefault("sources", {})["fine_dem"] = {
        "name": "Copernicus DEM GLO-90 (area average), ETOPO 2022 at sea and outside the bbox",
        "bbox_lonlat": list(copernicus.FINE_BBOX),
        "tiles": len(names),
    }
    path.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


def build(force: bool = False, map_dir: Path = MAP_DIR) -> ReliefShadeResult:
    """Download Copernicus if needed, then write the fine tiles, render heightmap and shade."""
    started = time.perf_counter()
    grid = relief.fine_grid(map_dir)
    factor = grid.size_px // 4096
    names = copernicus.tiles_in_bbox(copernicus.tile_list())
    copernicus.fetch_tiles(names)
    etopo = relief.resample_heights(
        grid, download.etopo_tiles(download.etopo_tiles_covering(*grid.geographic_extent()))
    )
    cop = copernicus_on_grid(
        grid, names, cache=None if force else CACHE_DIR / f"cop_{grid.size_px}.npy"
    )
    merged = copernicus.merge_with_etopo(cop, etopo)
    del cop, etopo
    original = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png")).astype(
        np.float32
    )
    land = upsample_sign(original, factor)
    fine = enforce_coast(boost_relief(merged, land, grid.meters_per_px), land)
    del merged
    encoded = terrain.height_to_uint16(fine)
    paths = relief.write_tiles(encoded, map_dir / relief.TILE_DIR)
    fine = terrain.uint16_to_height(encoded).astype(np.float32)
    del encoded
    render = block_mean(fine, factor)
    render = np.where(original > 0.0, render, original)
    render_path = map_dir / RENDER_HEIGHTMAP
    terrain.write_png16(terrain.height_to_uint16(render), render_path)
    detail = fine - bilinear_upsample(render, factor)
    occ = occlusion(fine, grid.meters_per_px)
    # Neutre en mer (le shader ne l'utilise que sur terre ; compression).
    detail[~land] = 0.0
    occ[~land] = 0.0
    shade_path = map_dir / RELIEF_SHADE
    Image.fromarray(encode_shade(detail, occ), mode="LA").save(shade_path, compress_level=9)
    relief.update_map_json(map_dir)
    update_map_json(map_dir, names)
    return ReliefShadeResult(
        tiles=len(paths),
        tiles_bytes=sum(p.stat().st_size for p in paths),
        render_heightmap=render_path,
        relief_shade=shade_path,
        shade_bytes=shade_path.stat().st_size,
        seconds=time.perf_counter() - started,
    )
