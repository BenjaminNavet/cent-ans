"""Copernicus DEM GLO-90 tiles: listing, download and resampling (lot R1).

Source: Copernicus DEM GLO-90 (© DLR e.V. 2010-2014 and © Airbus Defence and Space
GmbH 2014-2018, provided under COPERNICUS by the European Union and ESA; free
licence, attribution required), served anonymously over HTTPS from the public AWS
Open Data bucket ``copernicus-dem-90m`` (no cost, no account).

Tiles are 1° x 1° Cloud Optimised GeoTIFFs (EPSG:4326, 3 arc-seconds, heights in
metres above the EGM2008 geoid, like ETOPO 2022) named after their south-west
corner. Only land tiles exist: :data:`TILE_LIST_URL` lists them. Everything is
cached under ``tools/geo/raw/copernicus/`` (gitignored).

The playable land (France, British Isles, Low Countries, northern Spain, western
Germany, Switzerland, northern Italy) lies inside :data:`FINE_BBOX`; the rest of
the map extent keeps ETOPO 2022 (see :func:`merge_with_etopo`).
"""

from __future__ import annotations

import re
import time
from collections.abc import Iterable
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import numpy as np
import rasterio
from rasterio.enums import Resampling
from rasterio.warp import reproject
from rasterio.windows import from_bounds

from cent_ans_tools.geo import download
from cent_ans_tools.geo.project import CRS_GEO, CRS_MAP, MapGrid

BUCKET_URL = "https://copernicus-dem-90m.s3.amazonaws.com"
TILE_LIST_URL = f"{BUCKET_URL}/tileList.txt"
RAW_DIR = download.RAW_DIR / "copernicus"
#: ``(lon_min, lat_min, lon_max, lat_max)`` of the area rebuilt from Copernicus.
FINE_BBOX = (-11.0, 41.0, 12.0, 60.0)
DOWNLOAD_WORKERS = 6
FETCH_ATTEMPTS = 5

_TILE_RE = re.compile(r"Copernicus_DSM_COG_30_([NS])(\d{2})_00_([EW])(\d{3})_00_DEM")


def tile_name(lon_west: int, lat_south: int) -> str:
    """Name of the tile whose south-west corner is ``(lon_west, lat_south)``."""
    ns = "N" if lat_south >= 0 else "S"
    ew = "E" if lon_west >= 0 else "W"
    return f"Copernicus_DSM_COG_30_{ns}{abs(lat_south):02d}_00_{ew}{abs(lon_west):03d}_00_DEM"


def parse_tile_name(name: str) -> tuple[int, int] | None:
    """``(lon_west, lat_south)`` of a tile name, ``None`` if it does not match."""
    match = _TILE_RE.search(name)
    if match is None:
        return None
    lat = int(match[2]) * (1 if match[1] == "N" else -1)
    lon = int(match[4]) * (1 if match[3] == "E" else -1)
    return lon, lat


def tiles_in_bbox(
    names: Iterable[str], bbox: tuple[float, float, float, float] = FINE_BBOX
) -> list[str]:
    """Tile names (from the bucket list) intersecting ``bbox``, sorted."""
    lon_min, lat_min, lon_max, lat_max = bbox
    result = []
    for name in names:
        corner = parse_tile_name(name)
        if corner is None:
            continue
        lon, lat = corner
        if lon + 1 > lon_min and lon < lon_max and lat + 1 > lat_min and lat < lat_max:
            result.append(tile_name(lon, lat))
    return sorted(set(result))


def tile_list(force: bool = False) -> list[str]:
    """Names of every land tile of the bucket (cached ``tileList.txt``)."""
    path = download.download_file(TILE_LIST_URL, RAW_DIR / "tileList.txt", force)
    return [line.strip() for line in path.read_text().splitlines() if line.strip()]


def tile_path(name: str) -> Path:
    """Cached GeoTIFF path of a tile."""
    return RAW_DIR / f"{name}.tif"


def fetch_tiles(names: list[str], force: bool = False) -> list[Path]:
    """Download ``names`` in parallel (already cached tiles are skipped)."""

    def fetch(name: str) -> Path:
        url = f"{BUCKET_URL}/{name}/{name}.tif"
        for attempt in range(FETCH_ATTEMPTS):
            try:
                return download.download_file(url, tile_path(name), force)
            except download.DownloadError:
                if attempt == FETCH_ATTEMPTS - 1:
                    raise
                time.sleep(2.0 * (attempt + 1))
        return tile_path(name)

    with ThreadPoolExecutor(DOWNLOAD_WORKERS) as pool:
        return list(pool.map(fetch, names))


def resample_to_grid(
    grid: MapGrid,
    window: tuple[int, int, int, int],
    names: list[str],
    resampling: Resampling = Resampling.average,
) -> np.ndarray:
    """Copernicus heights on a window of ``grid`` (NaN where no tile covers it).

    Args:
        grid: Target grid (EPSG:3035).
        window: ``(col0, row0, cols, rows)`` in ``grid`` pixels.
        names: Candidate tile names (only those intersecting the window are read).
        resampling: Area average by default: the source (≈ 90 m) is finer than
            every target grid of the project, so averaging filters it correctly.

    Returns:
        ``float32`` array ``(rows, cols)``.
    """
    col0, row0, cols, rows = window
    transform = grid.transform * rasterio.Affine.translation(col0, row0)
    destination = np.full((rows, cols), np.nan, dtype=np.float32)
    lon_min, lon_max, lat_min, lat_max = _window_lonlat(grid, window)
    for name in names:
        corner = parse_tile_name(name)
        path = tile_path(name)
        if corner is None or not path.exists():
            continue
        lon, lat = corner
        if lon + 1 < lon_min or lon > lon_max or lat + 1 < lat_min or lat > lat_max:
            continue
        with rasterio.open(path) as source:
            win = from_bounds(
                max(lon, lon_min - 0.05),
                max(lat, lat_min - 0.05),
                min(lon + 1, lon_max + 0.05),
                min(lat + 1, lat_max + 0.05),
                source.transform,
            ).round_offsets().round_lengths()
            data = source.read(1, window=win, boundless=False).astype(np.float32)
            if data.size == 0:
                continue
            src_transform = source.window_transform(win)
        part = np.full((rows, cols), np.nan, dtype=np.float32)
        reproject(
            source=data,
            destination=part,
            src_transform=src_transform,
            src_crs=CRS_GEO,
            src_nodata=np.nan,
            dst_transform=transform,
            dst_crs=CRS_MAP,
            dst_nodata=np.nan,
            resampling=resampling,
        )
        filled = ~np.isnan(part)
        destination[filled] = part[filled]
    return destination


def _window_lonlat(
    grid: MapGrid, window: tuple[int, int, int, int]
) -> tuple[float, float, float, float]:
    """Geographic box ``(lon_min, lon_max, lat_min, lat_max)`` of a grid window."""
    from pyproj import Transformer

    col0, row0, cols, rows = window
    xs = np.linspace(col0, col0 + cols, 9)
    ys = np.linspace(row0, row0 + rows, 9)
    gx, gy = np.meshgrid(xs, ys)
    px, py = grid.transform * (gx.ravel(), gy.ravel())
    lon, lat = Transformer.from_crs(CRS_MAP, CRS_GEO, always_xy=True).transform(px, py)
    return float(np.min(lon)), float(np.max(lon)), float(np.min(lat)), float(np.max(lat))


def merge_with_etopo(copernicus_m: np.ndarray, etopo_m: np.ndarray) -> np.ndarray:
    """Copernicus on land, ETOPO elsewhere (sea bathymetry, areas without tiles).

    Copernicus stores the sea surface as 0 m: those pixels (and NaN) take the
    ETOPO value, which carries the bathymetry used by the water shader. A thin
    blend (where Copernicus is between 0 and 1 m) avoids steps on the shore.
    """
    valid = ~np.isnan(copernicus_m)
    cop = np.where(valid, copernicus_m, 0.0)
    land = valid & (np.abs(cop) > 0.01)
    return np.where(land, cop, etopo_m).astype(np.float32)
