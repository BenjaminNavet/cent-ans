"""Tier-2 relief sources: Copernicus DEM GLO-30 and ESA WorldCover (lot ZG1, ADR 0036).

Copernicus DEM GLO-30 (© DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH
2014-2018, provided under COPERNICUS by the European Union and ESA; free licence,
attribution required) is served anonymously over HTTPS from the public AWS Open
Data bucket ``copernicus-dem-30m``: 1° x 1° Cloud Optimised GeoTIFFs (1 arc-second
in latitude, 1 to 1.5 arc-seconds in longitude above 50°N, metres above EGM2008).

ESA WorldCover 10 m 2021 v200 (© ESA WorldCover project 2021 / Contains modified
Copernicus Sentinel data (2021) processed by ESA WorldCover consortium; CC BY 4.0)
is served from the public bucket ``esa-worldcover``: 3° x 3° COGs of land cover
classes (10 trees, 50 built-up, 80 permanent water...).

Both land in ``tools/geo/raw/`` (gitignored). No account, no cost.
"""

from __future__ import annotations

import math
import re
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from cent_ans_tools.geo import download

GLO30_BUCKET_URL = "https://copernicus-dem-30m.s3.amazonaws.com"
GLO30_TILE_LIST_URL = f"{GLO30_BUCKET_URL}/tileList.txt"
GLO30_DIR = download.RAW_DIR / "copernicus30"
WORLDCOVER_URL = "https://esa-worldcover.s3.eu-central-1.amazonaws.com/v200/2021/map"
WORLDCOVER_DIR = download.RAW_DIR / "worldcover"
WORLDCOVER_TILE_DEG = 3
DOWNLOAD_WORKERS = 6
FETCH_ATTEMPTS = 5

#: WorldCover classes used by the surface correction.
WC_TREES = 10
WC_BUILT = 50
WC_WATER = 80

_GLO30_RE = re.compile(r"Copernicus_DSM_COG_10_([NS])(\d{2})_00_([EW])(\d{3})_00_DEM")


def glo30_tile_name(lon_west: int, lat_south: int) -> str:
    """GLO-30 name of the 1° tile whose south-west corner is ``(lon_west, lat_south)``."""
    ns = "N" if lat_south >= 0 else "S"
    ew = "E" if lon_west >= 0 else "W"
    return f"Copernicus_DSM_COG_10_{ns}{abs(lat_south):02d}_00_{ew}{abs(lon_west):03d}_00_DEM"


def parse_glo30_name(name: str) -> tuple[int, int] | None:
    """``(lon_west, lat_south)`` of a GLO-30 tile name, ``None`` if it does not match."""
    match = _GLO30_RE.search(name)
    if match is None:
        return None
    lat = int(match[2]) * (1 if match[1] == "N" else -1)
    lon = int(match[4]) * (1 if match[3] == "E" else -1)
    return lon, lat


def glo30_tiles_in_bbox(
    names: list[str], bbox: tuple[float, float, float, float]
) -> list[str]:
    """Names (from the bucket list) of the GLO-30 tiles intersecting ``bbox``."""
    lon_min, lat_min, lon_max, lat_max = bbox
    result = set()
    for name in names:
        corner = parse_glo30_name(name)
        if corner is None:
            continue
        lon, lat = corner
        if lon + 1 > lon_min and lon < lon_max and lat + 1 > lat_min and lat < lat_max:
            result.add(glo30_tile_name(lon, lat))
    return sorted(result)


def glo30_tile_list(force: bool = False) -> list[str]:
    """Names of every GLO-30 land tile (cached ``tileList.txt``)."""
    path = download.download_file(
        GLO30_TILE_LIST_URL, GLO30_DIR / "tileList.txt", force
    )
    return [line.strip() for line in path.read_text().splitlines() if line.strip()]


def glo30_path(name: str) -> Path:
    """Cached GeoTIFF of a GLO-30 tile."""
    return GLO30_DIR / f"{name}.tif"


def worldcover_tile_name(lon_west: int, lat_south: int) -> str:
    """WorldCover tile covering ``[lon_west, lon_west + 3[ x [lat_south, lat_south + 3[``."""
    ns = "N" if lat_south >= 0 else "S"
    ew = "E" if lon_west >= 0 else "W"
    return (
        f"ESA_WorldCover_10m_2021_v200_{ns}{abs(lat_south):02d}"
        f"{ew}{abs(lon_west):03d}_Map"
    )


def worldcover_tiles_in_bbox(bbox: tuple[float, float, float, float]) -> list[str]:
    """WorldCover tile names intersecting ``bbox`` (tiles fully at sea may not exist)."""
    lon_min, lat_min, lon_max, lat_max = bbox
    step = WORLDCOVER_TILE_DEG
    names = []
    for lat in range(
        math.floor(lat_min / step) * step, math.ceil(lat_max / step) * step, step
    ):
        for lon in range(
            math.floor(lon_min / step) * step, math.ceil(lon_max / step) * step, step
        ):
            names.append(worldcover_tile_name(lon, lat))
    return names


def worldcover_path(name: str) -> Path:
    """Cached GeoTIFF of a WorldCover tile."""
    return WORLDCOVER_DIR / f"{name}.tif"


def worldcover_corner(name: str) -> tuple[int, int]:
    """``(lon_west, lat_south)`` of a WorldCover tile name."""
    match = re.search(r"_([NS])(\d{2})([EW])(\d{3})_Map", name)
    if match is None:
        raise ValueError(name)
    lat = int(match[2]) * (1 if match[1] == "N" else -1)
    lon = int(match[4]) * (1 if match[3] == "E" else -1)
    return lon, lat


def _fetch_all(jobs: list[tuple[str, Path]], force: bool) -> list[Path]:
    """Download ``(url, path)`` pairs in parallel; missing remote files are skipped."""

    def fetch(job: tuple[str, Path]) -> Path | None:
        url, path = job
        for attempt in range(FETCH_ATTEMPTS):
            try:
                return download.download_file(url, path, force)
            except download.DownloadError as error:
                if "HTTP 404" in str(error) or "HTTP 403" in str(error):
                    return None
                if attempt == FETCH_ATTEMPTS - 1:
                    raise
                time.sleep(2.0 * (attempt + 1))
        return None

    with ThreadPoolExecutor(DOWNLOAD_WORKERS) as pool:
        return [path for path in pool.map(fetch, jobs) if path is not None]


def fetch_glo30(names: list[str], force: bool = False) -> list[Path]:
    """Download GLO-30 tiles (cached ones skipped)."""
    return _fetch_all(
        [(f"{GLO30_BUCKET_URL}/{name}/{name}.tif", glo30_path(name)) for name in names],
        force,
    )


def fetch_worldcover(names: list[str], force: bool = False) -> list[Path]:
    """Download WorldCover tiles (cached ones skipped, tiles at sea do not exist)."""
    return _fetch_all(
        [(f"{WORLDCOVER_URL}/{name}.tif", worldcover_path(name)) for name in names],
        force,
    )


def raw_bytes() -> dict[str, int]:
    """Bytes cached per source directory."""
    return {
        directory.name: sum(p.stat().st_size for p in directory.glob("*.tif"))
        for directory in (GLO30_DIR, WORLDCOVER_DIR)
        if directory.exists()
    }
