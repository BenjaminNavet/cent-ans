"""8192² relief split into 16 x 16 tiles of 512² 16-bit PNGs (spec 2026-09-24 § 5).

Same extent (``bounds_projected``) and same height encoding as
``heightmap.png`` (see :mod:`cent_ans_tools.geo.terrain`): one 8192 pixel is
half a 4096 pixel (≈ 360 m). ETOPO 2022 15″ (≈ 300-460 m) is resampled
bilinearly: the target is about as fine as the source, so area averaging would
leave blocky steps.

Tile ``h_<col>_<row>.png`` covers pixels ``[col*512, (col+1)*512[`` in X and
``[row*512, (row+1)*512[`` in Y of the 8192² grid (no overlap: a renderer
stitching meshes reads the first row/column of the next tile). ``map.json``
gains ``height_tiles``; ``heightmap.png`` (4096²) is kept for compatibility.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from rasterio.enums import Resampling
from rasterio.warp import reproject

from cent_ans_tools.geo import download, terrain
from cent_ans_tools.geo.project import CRS_GEO, CRS_MAP, MapGrid, grid_from_metadata
from cent_ans_tools.paths import REPO_DIR

MAP_DIR = REPO_DIR / "data" / "map"
#: Fine relief is twice as fine as the map grid (one fine pixel = half a map pixel).
FINE_SCALE = 2
TILE_PX = 512
TILE_DIR = "height"
TILE_PATTERN = "h_{col}_{row}.png"


def height_tiles_meta(grid: MapGrid) -> dict:
    """``map.json.height_tiles`` for the fine grid ``grid`` ([w, h] in pixels)."""
    return {
        "size_px": [grid.width_px, grid.height_px],
        "tile_px": TILE_PX,
        "dir": TILE_DIR,
        "pattern": TILE_PATTERN,
    }


@dataclass
class ReliefResult:
    """Output of :func:`build`."""

    directory: Path
    tiles: int
    total_bytes: int
    seconds: float


def fine_grid(map_dir: Path = MAP_DIR) -> MapGrid:
    """The fine relief grid (twice the map grid) over the ``map.json`` bounds."""
    metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    return grid_from_metadata(metadata, FINE_SCALE)


def resample_heights(grid: MapGrid, tile_paths: list[Path]) -> np.ndarray:
    """ETOPO mosaic reprojected bilinearly onto ``grid`` (metres, NaN -> 0)."""
    mosaic, src_transform = terrain.mosaic_dem(tile_paths, grid.geographic_extent())
    destination = np.full(grid.shape, np.nan, dtype=np.float32)
    reproject(
        source=mosaic,
        destination=destination,
        src_transform=src_transform,
        src_crs=CRS_GEO,
        src_nodata=np.nan,
        dst_transform=grid.transform,
        dst_crs=CRS_MAP,
        dst_nodata=np.nan,
        resampling=Resampling.bilinear,
    )
    return np.nan_to_num(destination, nan=0.0)


def write_tiles(encoded: np.ndarray, directory: Path) -> list[Path]:
    """Split a ``uint16`` array into ``TILE_PX`` tiles named by :data:`TILE_PATTERN`."""
    directory.mkdir(parents=True, exist_ok=True)
    rows, cols = encoded.shape[0] // TILE_PX, encoded.shape[1] // TILE_PX
    paths = []
    for row in range(rows):
        for col in range(cols):
            block = encoded[
                row * TILE_PX : (row + 1) * TILE_PX, col * TILE_PX : (col + 1) * TILE_PX
            ]
            path = directory / TILE_PATTERN.format(col=col, row=row)
            terrain.write_png16(block, path)
            paths.append(path)
    return paths


def read_tiles(
    directory: Path, count: int | tuple[int, int], rows: int | None = None
) -> np.ndarray:
    """Reassemble tiles into one ``uint16`` array.

    Args:
        directory: Tile directory.
        count: Tiles per row (columns), or ``(cols, rows)``.
        rows: Tile rows (defaults to ``count``: square layout).
    """
    if isinstance(count, tuple):
        count, rows = count
    rows = count if rows is None else rows
    rows = [
        np.hstack(
            [
                terrain.read_png16(directory / TILE_PATTERN.format(col=col, row=row))
                for col in range(count)
            ]
        )
        for row in range(rows)
    ]
    return np.vstack(rows)


def tile_counts(map_dir: Path = MAP_DIR) -> tuple[int, int]:
    """``(cols, rows)`` of the fine relief tiles."""
    grid = fine_grid(map_dir)
    return grid.width_px // TILE_PX, grid.height_px // TILE_PX


def update_map_json(map_dir: Path) -> None:
    """Add (or refresh) ``height_tiles`` in ``map.json``, keeping every other key."""
    path = map_dir / "map.json"
    metadata = json.loads(path.read_text(encoding="utf-8"))
    metadata["height_tiles"] = height_tiles_meta(fine_grid(map_dir))
    path.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


def build(force: bool = False, map_dir: Path = MAP_DIR) -> ReliefResult:
    """Write ``data/map/height/h_<col>_<row>.png`` and ``map.json.height_tiles``."""
    started = time.perf_counter()
    grid = fine_grid(map_dir)
    names = download.etopo_tiles_for_grid(grid)
    heights = resample_heights(grid, download.etopo_tiles(names, force))
    paths = write_tiles(terrain.height_to_uint16(heights), map_dir / TILE_DIR)
    update_map_json(map_dir)
    return ReliefResult(
        directory=map_dir / TILE_DIR,
        tiles=len(paths),
        total_bytes=sum(p.stat().st_size for p in paths),
        seconds=time.perf_counter() - started,
    )
