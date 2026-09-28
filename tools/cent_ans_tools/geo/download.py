"""Download and cache the raw datasets used by the geo pipeline.

Everything lands in ``tools/geo/raw/`` (gitignored). Downloads are plain HTTPS
GETs without authentication; a file already present is never fetched again
unless ``force`` is set.

Sources:
    * Natural Earth 10m physical vectors (public domain):
      https://www.naturalearthdata.com/ served by https://naciscdn.org/
    * ETOPO 2022 (NOAA NCEI, public domain), 15 arc-second ice-surface
      elevation GeoTIFF tiles of 15 x 15 degrees named after their
      north-west corner:
      https://www.ncei.noaa.gov/products/etopo-global-relief-model
"""

from __future__ import annotations

import zipfile
from collections.abc import Iterable
from pathlib import Path

import httpx
import numpy as np

TOOLS_DIR = Path(__file__).resolve().parents[2]
RAW_DIR = TOOLS_DIR / "geo" / "raw"

NATURAL_EARTH_BASE = "https://naciscdn.org/naturalearth/10m/physical"
NATURAL_EARTH_LAYERS: dict[str, str] = {
    "coastline": "ne_10m_coastline",
    "land": "ne_10m_land",
    "rivers": "ne_10m_rivers_lake_centerlines",
    "rivers_europe": "ne_10m_rivers_europe",
    "lakes": "ne_10m_lakes",
    "lakes_europe": "ne_10m_lakes_europe",
    # Named deserts (Sahara, Arabian, Syrian, Karakum...): land cover of the
    # East and South (ADR 0115), where no finer land-cover source is used.
    "geography_regions": "ne_10m_geography_regions_polys",
}

ETOPO_BASE = "https://www.ngdc.noaa.gov/mgg/global/relief/ETOPO2022/data/15s/15s_surface_elev_gtif"
ETOPO_TILE_DEG = 15


class DownloadError(RuntimeError):
    """Raised when a remote file cannot be fetched."""


def download_file(url: str, target: Path, force: bool = False) -> Path:
    """Stream ``url`` into ``target`` unless it already exists.

    Args:
        url: HTTPS address of the file.
        target: Destination path; parent directories are created.
        force: Re-download even if ``target`` exists.

    Returns:
        The path of the cached file.

    Raises:
        DownloadError: On HTTP errors.
    """
    if target.exists() and not force:
        return target
    target.parent.mkdir(parents=True, exist_ok=True)
    partial = target.with_suffix(target.suffix + ".part")
    try:
        with (
            httpx.stream("GET", url, follow_redirects=True, timeout=120.0) as response,
            partial.open("wb") as handle,
        ):
            if response.status_code != 200:
                raise DownloadError(f"{url}: HTTP {response.status_code}")
            for chunk in response.iter_bytes(1 << 20):
                handle.write(chunk)
    except httpx.HTTPError as error:
        partial.unlink(missing_ok=True)
        raise DownloadError(f"{url}: {error}") from error
    partial.replace(target)
    return target


def natural_earth_shapefile(layer: str, force: bool = False) -> Path:
    """Fetch a Natural Earth 10m layer and return the extracted ``.shp`` path.

    Args:
        layer: Key of :data:`NATURAL_EARTH_LAYERS`.
        force: Re-download the archive.
    """
    stem = NATURAL_EARTH_LAYERS[layer]
    archive = download_file(
        f"{NATURAL_EARTH_BASE}/{stem}.zip",
        RAW_DIR / "natural_earth" / f"{stem}.zip",
        force,
    )
    extract_dir = archive.parent / stem
    shapefile = extract_dir / f"{stem}.shp"
    if not shapefile.exists() or force:
        with zipfile.ZipFile(archive) as zipped:
            zipped.extractall(extract_dir)
    return shapefile


def etopo_tile_name(lon_west: int, lat_north: int) -> str:
    """Return the ETOPO tile file name whose north-west corner is given.

    Args:
        lon_west: Western longitude of the tile (multiple of 15).
        lat_north: Northern latitude of the tile (multiple of 15).
    """
    ns = "N" if lat_north >= 0 else "S"
    ew = "E" if lon_west >= 0 else "W"
    return (
        f"ETOPO_2022_v1_15s_{ns}{abs(lat_north):02d}{ew}{abs(lon_west):03d}_surface.tif"
    )


def etopo_tiles_covering(
    lon_min: float, lon_max: float, lat_min: float, lat_max: float
) -> list[str]:
    """List the 15-degree ETOPO tiles intersecting a geographic extent.

    Args:
        lon_min: Western bound in degrees.
        lon_max: Eastern bound in degrees.
        lat_min: Southern bound in degrees.
        lat_max: Northern bound in degrees.

    Returns:
        Tile file names, west to east then north to south.
    """
    step = ETOPO_TILE_DEG
    first_lon = int(lon_min // step) * step
    last_lon = int((lon_max - 1e-9) // step) * step
    first_lat = int(lat_min // step) * step
    last_lat = int((lat_max - 1e-9) // step) * step
    names: list[str] = []
    for lat_south in range(last_lat, first_lat - 1, -step):
        for lon_west in range(first_lon, last_lon + 1, step):
            names.append(etopo_tile_name(lon_west, lat_south + step))
    return names


def etopo_tiles(names: Iterable[str], force: bool = False) -> list[Path]:
    """Fetch ETOPO tiles by name and return their cached paths."""
    return [
        download_file(f"{ETOPO_BASE}/{name}", RAW_DIR / "etopo2022" / name, force)
        for name in names
    ]


def etopo_tiles_for_grid(grid: object, samples: int = 400) -> list[str]:
    """ETOPO tiles that actually intersect a map grid's projected rectangle.

    The lon/lat bounding box of a large EPSG:3035 rectangle is much wider than
    the rectangle itself (its edges are curved): tiles are kept only if they
    intersect the rectangle's densified outline polygon.

    Args:
        grid: A :class:`~cent_ans_tools.geo.project.MapGrid`.
        samples: Points sampled along each edge of the rectangle.

    Returns:
        Tile file names, north to south then west to east.
    """
    from shapely.geometry import Polygon, box

    from cent_ans_tools.geo.project import _to_geo

    minx, miny, maxx, maxy = grid.bounds
    xs = np.linspace(minx, maxx, samples)
    ys = np.linspace(miny, maxy, samples)
    ring_x = np.concatenate([xs, np.full(samples, maxx), xs[::-1], np.full(samples, minx)])
    ring_y = np.concatenate([np.full(samples, miny), ys, np.full(samples, maxy), ys[::-1]])
    lon, lat = _to_geo.transform(ring_x, ring_y)
    outline = Polygon(zip(lon, lat, strict=True)).buffer(0)
    step = ETOPO_TILE_DEG
    return [
        name
        for name in etopo_tiles_covering(*grid.geographic_extent())
        if outline.intersects(box(*_tile_box(name, step)))
    ]


def _tile_box(name: str, step: int) -> tuple[float, float, float, float]:
    """``(lon_min, lat_min, lon_max, lat_max)`` of an ETOPO tile name."""
    code = name.split("_")[4]
    lat = int(code[1:3]) * (1 if code[0] == "N" else -1)
    lon = int(code[4:7]) * (1 if code[3] == "E" else -1)
    return (lon, lat - step, lon + step, lat)
