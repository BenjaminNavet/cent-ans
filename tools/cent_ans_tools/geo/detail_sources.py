"""Bare-earth DTM fetchers of the relief pyramid, tier 3 (lot ZG3, ADR 0036).

Every source is free, needs no key and was checked by hand on 2026-09-25:

``ign_rge_alti``
    IGN RGE ALTI 1 m / 5 m (France), Géoplateforme WMS-R
    ``https://data.geopf.fr/wms-r/wms``, layer
    ``ELEVATION.ELEVATIONGRIDCOVERAGE.HIGHRES``, ``image/geotiff`` float32
    (NGF-IGN69 metres, nodata -99999), EPSG:2154, at most 5010 px a side.
    Licence Ouverte Etalab 2.0.
``ea_lidar``
    Environment Agency LIDAR Composite DTM 1 m (England), WCS 2.0.1
    ``https://environment.data.gov.uk/spatialdata/lidar-composite-digital-terrain-model-dtm-1m/wcs``,
    EPSG:27700 (ODN metres), ``scalefactor`` honoured. Open Government Licence v3.
``ahn``
    Actueel Hoogtebestand Nederland DTM 0.5 m (Netherlands), PDOK WCS 2.0.1
    ``https://service.pdok.nl/rws/ahn/wcs/v1_0`` coverage ``dtm_05m``, EPSG:28992
    (NAP metres), ``scalesize`` honoured, slow (~20 s a request). CC0 1.0.
``dhm_vlaanderen``
    Digitaal Hoogtemodel Vlaanderen (Flanders), WCS 2.0.1
    ``https://geo.api.vlaanderen.be/DHMV/wcs``: ``DHMVI_DTM_5m`` for E5,
    ``DHMVII_DTM_1m`` for E6-E7; EPSG:31370 (TAW metres, ~2.33 m above NAP),
    multipart GML + TIFF answer, no server-side scaling, at most ~2000 px a side
    (the 1 m chunks are block-averaged here before caching). Modellicentie
    Gratis Hergebruik v1.0 (Digitaal Vlaanderen).
``glo30``
    Fallback: Copernicus DEM GLO-30 (surface model, not corrected here), shared
    cache ``tools/geo/raw/copernicus30/`` with lot ZG1.

Wallonia (SPW) only publishes its DTM as rendered WMS/MapServer images (no WCS,
no raw values), so Walloon zones use the GLO-30 fallback.

Chunks are cached as compressed float32 GeoTIFFs in their native CRS under
``tools/geo/raw/detail/<zone>/E<level>/``; an existing chunk is never fetched
again (resume after interruption). Requests are rate limited per host, retried
with exponential back-off and carry a User-Agent naming the project.
"""

from __future__ import annotations

import io
import math
import threading
import time
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import httpx
import numpy as np
import rasterio
from pyproj import Transformer

from cent_ans_tools.geo import download

DETAIL_RAW_DIR = download.RAW_DIR / "detail"
GLO30_DIR = download.RAW_DIR / "copernicus30"
GLO30_BUCKET_URL = "https://copernicus-dem-30m.s3.amazonaws.com"
USER_AGENT = (
    "cent-ans-tools/0.1 (Cent Ans, open historical strategy game; "
    "one-off relief bake, rate limited)"
)
ATTEMPTS = 6
#: Heights outside this range are nodata (sources use ±3.4e38, -9999, -99999).
VALID_RANGE_M = (-150.0, 5000.0)


@dataclass(frozen=True)
class Source:
    """One DTM service.

    Attributes:
        key: Identifier used in ``detail_zones.json``.
        label: Human name (manifest, docs).
        crs: Native CRS of the requests.
        native_m: Native pixel size in metres.
        max_px: Largest request side in pixels.
        host: Host name (rate limiting).
        min_interval_s: Minimum delay between two requests to the host.
        datum_offset_m: Added to the heights to bring them near mean sea level
            (EGM2008 / NAP / ODN / NGF-IGN69 agree within a metre).
        coverage: ``(minx, miny, maxx, maxy)`` of the service in ``crs``.
    """

    key: str
    label: str
    crs: str
    native_m: float
    max_px: int
    host: str
    min_interval_s: float
    datum_offset_m: float = 0.0
    coverage: tuple[float, float, float, float] | None = None


SOURCES: dict[str, Source] = {
    "ign_rge_alti": Source(
        "ign_rge_alti",
        "IGN RGE ALTI 1 m (Géoplateforme WMS-R)",
        "EPSG:2154",
        1.0,
        4000,
        "data.geopf.fr",
        0.25,
        coverage=(60000.0, 6040000.0, 1250000.0, 7120000.0),
    ),
    "ea_lidar": Source(
        "ea_lidar",
        "Environment Agency LIDAR Composite DTM 1 m (WCS)",
        "EPSG:27700",
        1.0,
        4000,
        "environment.data.gov.uk",
        0.5,
        coverage=(80000.0, 4000.0, 656000.0, 665000.0),
    ),
    "ahn": Source(
        "ahn",
        "AHN DTM 0,5 m (PDOK WCS)",
        "EPSG:28992",
        0.5,
        4000,
        "service.pdok.nl",
        0.5,
        coverage=(10000.0, 306250.0, 280000.0, 618750.0),
    ),
    "dhm_vlaanderen": Source(
        "dhm_vlaanderen",
        "DHM Vlaanderen II DTM 1 m / I DTM 5 m (WCS)",
        "EPSG:31370",
        1.0,
        2000,
        "geo.api.vlaanderen.be",
        0.5,
        datum_offset_m=-2.33,
        coverage=(17000.0, 148000.0, 264000.0, 250000.0),
    ),
    "glo30": Source(
        "glo30",
        "Copernicus DEM GLO-30 (repli, modèle de surface)",
        "EPSG:4326",
        30.0,
        3600,
        "copernicus-dem-30m.s3.amazonaws.com",
        0.2,
    ),
}


class _HostGate:
    """Per-host concurrency (one request at a time) and minimum interval."""

    def __init__(self) -> None:
        self._locks: dict[str, threading.Lock] = {}
        self._last: dict[str, float] = {}
        self._guard = threading.Lock()

    def lock(self, host: str) -> threading.Lock:
        with self._guard:
            return self._locks.setdefault(host, threading.Lock())

    def wait(self, host: str, interval: float) -> None:
        elapsed = time.monotonic() - self._last.get(host, 0.0)
        if elapsed < interval:
            time.sleep(interval - elapsed)

    def done(self, host: str) -> None:
        self._last[host] = time.monotonic()


_GATE = _HostGate()


class FetchError(RuntimeError):
    """A chunk could not be fetched after every retry."""


def http_get(
    url: str, host: str, min_interval_s: float, timeout: float = 300.0
) -> bytes:
    """GET ``url`` politely: one request at a time per host, retries with back-off."""
    delay = 2.0
    last_error = ""
    for attempt in range(ATTEMPTS):
        with _GATE.lock(host):
            _GATE.wait(host, min_interval_s)
            try:
                response = httpx.get(
                    url,
                    timeout=timeout,
                    follow_redirects=True,
                    headers={"User-Agent": USER_AGENT},
                )
                status, body = response.status_code, response.content
            except httpx.HTTPError as error:
                status, body, last_error = 0, b"", str(error)
            finally:
                _GATE.done(host)
        if status == 200:
            return body
        if status:
            last_error = f"HTTP {status}: {body[:200]!r}"
        if status in (400, 401, 403, 404) and attempt >= 1:
            break
        if attempt < ATTEMPTS - 1:
            time.sleep(delay)
            delay *= 2.0
    raise FetchError(f"{url}: {last_error}")


def extract_tiff(body: bytes) -> bytes:
    """The TIFF part of a response (plain TIFF or WCS multipart GML + TIFF)."""
    for magic in (b"II*\x00", b"MM\x00*"):
        index = body.find(magic)
        if index >= 0:
            return body[index:]
    raise FetchError(f"no TIFF in the answer: {body[:300]!r}")


def request_url(
    source: Source,
    bbox: tuple[float, float, float, float],
    size: tuple[int, int],
    level: int,
) -> str:
    """URL of one chunk ``bbox`` (native CRS) rendered as ``size`` = (width, height) pixels."""
    minx, miny, maxx, maxy = bbox
    width, height = size
    if source.key == "ign_rge_alti":
        return (
            "https://data.geopf.fr/wms-r/wms?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap"
            "&LAYERS=ELEVATION.ELEVATIONGRIDCOVERAGE.HIGHRES&STYLES=&CRS=EPSG:2154"
            f"&BBOX={minx:.2f},{miny:.2f},{maxx:.2f},{maxy:.2f}"
            f"&WIDTH={width}&HEIGHT={height}&FORMAT=image/geotiff"
        )
    if source.key == "ea_lidar":
        base = (
            "https://environment.data.gov.uk/spatialdata/"
            "lidar-composite-" + "digital-terrain-model-dtm-1m/wcs"
        )
        scale = width / (maxx - minx) * source.native_m
        url = (
            f"{base}?service=WCS&request=GetCoverage&version=2.0.1"
            "&coverageId=13787b9a-26a4-4775-8523-806d13af58fc__Lidar_Composite_Elevation_DTM_1m"
            f"&format=image/tiff&subset=E({minx:.0f},{maxx:.0f})&subset=N({miny:.0f},{maxy:.0f})"
        )
        if scale < 0.999:
            url += f"&scalefactor={scale:.6f}"
        return url
    if source.key == "ahn":
        return (
            "https://service.pdok.nl/rws/ahn/wcs/v1_0?service=WCS&request=GetCoverage"
            "&version=2.0.1&coverageId=dtm_05m&format=image/tiff"
            f"&subset=x({minx:.1f},{maxx:.1f})&subset=y({miny:.1f},{maxy:.1f})"
            f"&scalesize=x({width}),y({height})"
        )
    if source.key == "dhm_vlaanderen":
        coverage = "DHMVI_DTM_5m" if level <= 5 else "DHMVII_DTM_1m"
        return (
            "https://geo.api.vlaanderen.be/DHMV/wcs?service=WCS&request=GetCoverage"
            f"&version=2.0.1&coverageId={coverage}&format=image/tiff"
            f"&subset=x({minx:.0f},{maxx:.0f})&subset=y({miny:.0f},{maxy:.0f})"
        )
    raise ValueError(source.key)


def native_request_m(source: Source, level: int, target_m: float) -> float:
    """Pixel size (m) asked of ``source``: half the level pixel, never finer than native."""
    if source.key == "dhm_vlaanderen":
        return 5.0 if level <= 5 else 1.0
    return max(source.native_m, target_m / 2.0)


def plan_chunks(
    source: Source, bbox: tuple[float, float, float, float], pixel_m: float
) -> list[tuple[tuple[float, float, float, float], tuple[int, int]]]:
    """Split a native-CRS ``bbox`` into requests of at most ``source.max_px`` a side.

    The box is snapped outwards to multiples of ``pixel_m`` so that the chunks
    tile it exactly. Returns ``[(chunk_bbox, (width, height)), ...]``.
    """
    minx = math.floor(bbox[0] / pixel_m) * pixel_m
    miny = math.floor(bbox[1] / pixel_m) * pixel_m
    maxx = math.ceil(bbox[2] / pixel_m) * pixel_m
    maxy = math.ceil(bbox[3] / pixel_m) * pixel_m
    if source.coverage is not None:
        cminx, cminy, cmaxx, cmaxy = source.coverage
        minx, miny = max(minx, cminx), max(miny, cminy)
        maxx, maxy = min(maxx, cmaxx), min(maxy, cmaxy)
        if minx >= maxx or miny >= maxy:
            return []
    cols = round((maxx - minx) / pixel_m)
    rows = round((maxy - miny) / pixel_m)
    step = source.max_px
    chunks = []
    for row0 in range(0, rows, step):
        for col0 in range(0, cols, step):
            width = min(step, cols - col0)
            height = min(step, rows - row0)
            x0 = minx + col0 * pixel_m
            y1 = maxy - row0 * pixel_m
            chunks.append(
                ((x0, y1 - height * pixel_m, x0 + width * pixel_m, y1), (width, height))
            )
    return chunks


def native_bbox(
    source: Source, bbox_3035: tuple[float, float, float, float], margin_m: float
) -> tuple[float, float, float, float]:
    """Bounding box in ``source.crs`` of an EPSG:3035 box grown by ``margin_m``."""
    minx, miny, maxx, maxy = bbox_3035
    xs = np.linspace(minx - margin_m, maxx + margin_m, 21)
    ys = np.linspace(miny - margin_m, maxy + margin_m, 21)
    edge_x = np.concatenate([xs, xs, np.full(21, xs[0]), np.full(21, xs[-1])])
    edge_y = np.concatenate([np.full(21, ys[0]), np.full(21, ys[-1]), ys, ys])
    transformer = Transformer.from_crs("EPSG:3035", source.crs, always_xy=True)
    x, y = transformer.transform(edge_x, edge_y)
    return float(np.min(x)), float(np.min(y)), float(np.max(x)), float(np.max(y))


def write_chunk(
    data: np.ndarray, transform: rasterio.Affine, crs: str, path: Path
) -> None:
    """Cache a float32 chunk (NaN = nodata) as a compressed GeoTIFF, atomically."""
    path.parent.mkdir(parents=True, exist_ok=True)
    partial = path.with_suffix(".part.tif")
    profile = {
        "driver": "GTiff",
        "width": data.shape[1],
        "height": data.shape[0],
        "count": 1,
        "dtype": "float32",
        "crs": crs,
        "transform": transform,
        "nodata": float("nan"),
        "compress": "deflate",
        "predictor": 3,
        "zlevel": 6,
        "tiled": True,
        "blockxsize": 256,
        "blockysize": 256,
    }
    with rasterio.open(partial, "w", **profile) as target:
        target.write(data.astype(np.float32), 1)
    partial.replace(path)


def clean_heights(data: np.ndarray, nodata: float | None) -> np.ndarray:
    """Float32 heights with every nodata flavour turned into NaN."""
    data = data.astype(np.float32)
    invalid = ~np.isfinite(data)
    if nodata is not None and np.isfinite(nodata):
        invalid |= data == np.float32(nodata)
    invalid |= (data < VALID_RANGE_M[0]) | (data > VALID_RANGE_M[1])
    data[invalid] = np.nan
    return data


def block_reduce_mean(data: np.ndarray, factor: int) -> np.ndarray:
    """NaN-aware mean of ``factor`` x ``factor`` blocks (partial edge blocks dropped)."""
    if factor <= 1:
        return data
    rows, cols = data.shape[0] // factor, data.shape[1] // factor
    view = data[: rows * factor, : cols * factor].reshape(rows, factor, cols, factor)
    valid = np.isfinite(view)
    total = np.where(valid, view, 0.0).sum(axis=(1, 3))
    count = valid.sum(axis=(1, 3))
    with np.errstate(invalid="ignore", divide="ignore"):
        mean = total / count
    mean[count * 2 < factor * factor] = np.nan
    return mean.astype(np.float32)


def fetch_chunk(
    source: Source,
    level: int,
    bbox: tuple[float, float, float, float],
    size: tuple[int, int],
    path: Path,
    target_m: float,
) -> Path:
    """Fetch one chunk into ``path`` (skipped when cached)."""
    if path.exists():
        return path
    if source.key == "dhm_vlaanderen" and level > 5:
        return _fetch_flanders_fine(source, bbox, size, path, target_m)
    url = request_url(source, bbox, size, level)
    body = extract_tiff(http_get(url, source.host, source.min_interval_s))
    with rasterio.open(io.BytesIO(body)) as dataset:
        data = clean_heights(dataset.read(1), dataset.nodata)
        transform = dataset.transform
    # Some servers answer in a grid of their own: keep their georeferencing.
    write_chunk(data, transform, source.crs, path)
    return path


def _fetch_flanders_fine(
    source: Source,
    bbox: tuple[float, float, float, float],
    size: tuple[int, int],
    path: Path,
    target_m: float,
) -> Path:
    """DHMV II 1 m chunk, block-averaged to about half the level pixel before caching."""
    url = request_url(source, bbox, size, 6)
    body = extract_tiff(http_get(url, source.host, source.min_interval_s))
    with rasterio.open(io.BytesIO(body)) as dataset:
        data = clean_heights(dataset.read(1), dataset.nodata)
        transform = dataset.transform
    factor = max(1, int(target_m / 2.0 / source.native_m))
    reduced = block_reduce_mean(data, factor)
    write_chunk(reduced, transform * rasterio.Affine.scale(factor), source.crs, path)
    return path


def fetch_zone_level(
    source_key: str,
    zone_id: str,
    level: int,
    bbox_3035: tuple[float, float, float, float],
    target_m: float,
    submit: Callable | None = None,
) -> list[Path]:
    """Fetch every chunk of ``source_key`` covering ``bbox_3035`` for one zone and level.

    Args:
        source_key: Key of :data:`SOURCES`.
        zone_id: Zone identifier (cache folder).
        level: Pyramid level (5-7).
        bbox_3035: Footprint to cover, EPSG:3035 metres.
        target_m: Pixel size of the level.
        submit: Optional ``executor.submit``-like callable to fetch chunks
            concurrently (the host gate still serialises requests per host).

    Returns:
        Cached chunk paths (existing files included).
    """
    if source_key not in SOURCES:
        raise FetchError(f"{source_key}: no raw-value service (rendered images only)")
    source = SOURCES[source_key]
    if source.key == "glo30":
        return glo30_tiles(bbox_3035)
    pixel_m = native_request_m(source, level, target_m)
    margin = 4.0 * target_m + 2.0 * pixel_m
    bbox = native_bbox(source, bbox_3035, margin)
    request_m = pixel_m
    if source.key == "dhm_vlaanderen" and level > 5:
        request_m = 1.0
    chunks = plan_chunks(source, bbox, request_m)
    folder = DETAIL_RAW_DIR / zone_id / f"E{level}"
    jobs = []
    for index, (chunk_bbox, size) in enumerate(chunks):
        path = folder / f"{source.key}_{index:03d}.tif"
        args = (source, level, chunk_bbox, size, path, target_m)
        jobs.append(submit(fetch_chunk, *args) if submit else fetch_chunk(*args))
    return [job.result() if hasattr(job, "result") else job for job in jobs]


def glo30_tile_name(lon_west: int, lat_south: int) -> str:
    """GLO-30 name of the 1° tile with south-west corner ``(lon_west, lat_south)`` (as ZG1)."""
    ns = "N" if lat_south >= 0 else "S"
    ew = "E" if lon_west >= 0 else "W"
    return f"Copernicus_DSM_COG_10_{ns}{abs(lat_south):02d}_00_{ew}{abs(lon_west):03d}_00_DEM"


def glo30_tiles(bbox_3035: tuple[float, float, float, float]) -> list[Path]:
    """GLO-30 tiles covering an EPSG:3035 box (downloaded into the ZG1 cache if missing)."""
    transformer = Transformer.from_crs("EPSG:3035", "EPSG:4326", always_xy=True)
    minx, miny, maxx, maxy = bbox_3035
    xs = np.linspace(minx, maxx, 11)
    ys = np.linspace(miny, maxy, 11)
    gx, gy = np.meshgrid(xs, ys)
    lon, lat = transformer.transform(gx.ravel(), gy.ravel())
    paths = []
    for lat_south in range(math.floor(lat.min()), math.floor(lat.max()) + 1):
        for lon_west in range(math.floor(lon.min()), math.floor(lon.max()) + 1):
            name = glo30_tile_name(lon_west, lat_south)
            path = GLO30_DIR / f"{name}.tif"
            if not path.exists():
                try:
                    download.download_file(
                        f"{GLO30_BUCKET_URL}/{name}/{name}.tif", path
                    )
                except download.DownloadError:
                    continue  # sea tile: none in the bucket
            paths.append(path)
    return paths
