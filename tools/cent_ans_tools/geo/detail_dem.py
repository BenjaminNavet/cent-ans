"""Relief pyramid, tier 3: national DTMs on the detail zones (lot ZG3, ADR 0036).

Reads ``data/map/detail_zones.json``, fetches bare-earth DTMs (IGN RGE ALTI,
Environment Agency LiDAR, AHN, DHM Vlaanderen; GLO-30 fallback, see
:mod:`cent_ans_tools.geo.detail_sources`) into ``tools/geo/raw/detail/``, erases
modern features (see :mod:`cent_ans_tools.geo.anachronisms`), then bakes levels
E5-E7 of the pyramid and rewrites their lines of ``data/map/relief_pyramid.json``.

Grid (ADR 0036): level ``k`` has ``16 * 2**k`` tiles of 512² pixels a side over
the square EPSG:3035 bounds of ``map.json``; pixel ``(i, j)`` of level ``k``
covers ``[minx + i * m, minx + (i + 1) * m]`` with ``m = 359.49 / 2**k`` metres
(pixel-centred values). Tile ``(k, col, row)`` has children
``(k + 1, 2 col + dx, 2 row + dy)``.

Footprints: level 5 covers the zone square (``half_size_km``), level 6
``min(half, 6 km)``, level 7 ``min(half, 3 km)`` unless the zone overrides them
(``level_half_km``). A tile exists wherever it meets a footprint. Inside a tile,
the baked height is ``w * fine + (1 - w) * ancestor`` where ``ancestor`` is the
bilinear upsampling of the finest existing coarser tile (the engine's own
fallback), ``w`` rises from 0 on the footprint edge to 1 over
:data:`FADE_FRACTION` of the half-size and drops to 0 over :data:`COAST_FADE_PX`
pixels where the source has no data (sea): there is no step at the zone edge
nor on the shore.

Render boost (ADR 0019, :func:`relief_shade.boost_relief`): the same unsharp
mask as ``heightmap_render.png`` (σ 5 km, gain 0.8, ±120 m, faded out above
600-1600 m), with the σ 5 km base computed at 90 m on GLO-90 around the zone
(the fine source replacing it inside) so that it matches the coarser levels.

After a level is baked, every parent tile (level ``k - 1``) of a baked tile is
set to the 2 x 2 mean of its children where they carry fine data (weighted by
``w``): a child averaged 2 x 2 gives back its parent to quantisation.
"""

from __future__ import annotations

import hashlib
import json
import math
import os
import re
import time
from concurrent.futures import ProcessPoolExecutor, ThreadPoolExecutor
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import rasterio
from pyproj import Transformer
from rasterio.enums import Resampling
from rasterio.transform import Affine
from rasterio.warp import reproject
from scipy import ndimage

from cent_ans_tools.geo import (
    anachronisms,
    bake_stamp,
    copernicus,
    detail_sources,
    terrain,
)
from cent_ans_tools.geo.detail_sources import DETAIL_RAW_DIR, SOURCES
from cent_ans_tools.geo.project import MapGrid
from cent_ans_tools.geo.relief_shade import (
    BOOST_FADE_M,
    BOOST_GAIN,
    BOOST_LIMIT_M,
    BOOST_SIGMA_M,
    floor_valleys,
)

REPO_DIR = detail_sources.download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
ZONES_FILE = MAP_DIR / "detail_zones.json"
MANIFEST_FILE = MAP_DIR / "relief_pyramid.json"
PREVIEW_DIR = REPO_DIR / "docs" / "img" / "zg3"
PYRAMID_DIR_NAME = "pyramid"
E0_DIR_NAME = "height"
TILE_PX = 512
ROOT_TILE_UNITS = 256
ROOT_TILES = 16
LEVELS = (5, 6, 7)
#: Default cap (km) of the footprint half-size per level.
LEVEL_CAP_KM = {5: 40.0, 6: 6.0, 7: 3.0}
FADE_FRACTION = 0.2
COAST_FADE_PX = 2
#: Nodata holes up to this area (m²) inside a zone are filled (rivers, ponds).
HOLE_MAX_M2 = 250_000.0
BASE_LEVEL = 2  # 90 m grid of the boost base
BASE_MARGIN_M = 3.0 * BOOST_SIGMA_M
MIN_LAND_M = 0.5
WORKERS = max(1, min(8, (os.cpu_count() or 4) - 2))
#: Bump when the bake changes, so that ``done`` markers are invalidated.
#: 5 (SZ2): valley floors not dug (:func:`relief_shade.valley_floor`), E1-E4 re-baked.
BAKE_VERSION = 5
#: Grey-opening width (GLO-30 pixels) turning the surface model into rough ground.
GLO30_OPENING_PX = 5
PREVIEW_ZONES = ("calais", "poitiers", "chateau_gaillard")

_TO_MAP = Transformer.from_crs("EPSG:4326", "EPSG:3035", always_xy=True)
_TO_GEO = Transformer.from_crs("EPSG:3035", "EPSG:4326", always_xy=True)


# --------------------------------------------------------------------------- zones


@dataclass(frozen=True)
class Zone:
    """One detail zone of ``detail_zones.json``."""

    id: str
    name: str
    kind: str
    lon: float
    lat: float
    half_size_km: float
    max_level: int
    source: str
    extra_sources: tuple[str, ...] = ()
    level_half_km: dict[int, float] = field(default_factory=dict)

    @property
    def center_3035(self) -> tuple[float, float]:
        """Centre in EPSG:3035 metres."""
        x, y = _TO_MAP.transform(self.lon, self.lat)
        return float(x), float(y)

    def sources(self) -> tuple[str, ...]:
        """Sources in priority order (the first valid pixel wins)."""
        return (self.source, *self.extra_sources)


def load_zones(path: Path = ZONES_FILE) -> list[Zone]:
    """Zones of ``detail_zones.json``."""
    document = json.loads(path.read_text(encoding="utf-8"))
    zones = []
    for item in document["zones"]:
        zones.append(
            Zone(
                id=item["id"],
                name=item["name"],
                kind=item["kind"],
                lon=float(item["center_lonlat"][0]),
                lat=float(item["center_lonlat"][1]),
                half_size_km=float(item["half_size_km"]),
                max_level=int(item["max_level"]),
                source=item["source"],
                extra_sources=tuple(item.get("extra_sources", ())),
                level_half_km={
                    int(k): float(v) for k, v in item.get("level_half_km", {}).items()
                },
            )
        )
    return zones


def level_half_km(zone: Zone, level: int) -> float | None:
    """Half-size (km) of the zone footprint at ``level``, ``None`` above ``max_level``."""
    if level > zone.max_level or level < LEVELS[0]:
        return None
    if level in zone.level_half_km:
        return zone.level_half_km[level]
    return min(zone.half_size_km, LEVEL_CAP_KM[level])


def footprint(zone: Zone, level: int) -> tuple[float, float, float, float] | None:
    """EPSG:3035 square ``(minx, miny, maxx, maxy)`` of the zone at ``level``."""
    half = level_half_km(zone, level)
    if half is None:
        return None
    cx, cy = zone.center_3035
    r = half * 1000.0
    return cx - r, cy - r, cx + r, cy + r


# ---------------------------------------------------------------------------- grid


@dataclass(frozen=True)
class PyramidGrid:
    """The pyramid grid of ``map.json`` (bounds and world unit)."""

    minx: float
    maxy: float
    unit_m: float

    @classmethod
    def from_map(cls, map_dir: Path = MAP_DIR) -> PyramidGrid:
        """Grid of ``map_dir/map.json``."""
        metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
        minx, _, _, maxy = metadata["bounds_projected"]
        return cls(float(minx), float(maxy), float(metadata["meters_per_px"]))

    def pixel_m(self, level: int) -> float:
        """Metres per pixel of ``level``."""
        return self.unit_m * ROOT_TILE_UNITS / (2**level) / TILE_PX

    def tiles_per_side(self, level: int) -> int:
        """Number of tiles along one side at ``level``."""
        return ROOT_TILES * 2**level

    def tile_range(
        self, level: int, bbox: tuple[float, float, float, float]
    ) -> tuple[int, int, int, int]:
        """Inclusive ``(col0, row0, col1, row1)`` of the tiles meeting ``bbox``."""
        size = self.pixel_m(level) * TILE_PX
        last = self.tiles_per_side(level) - 1
        minx, miny, maxx, maxy = bbox
        col0 = int(math.floor((minx - self.minx) / size))
        col1 = int(math.ceil((maxx - self.minx) / size)) - 1
        row0 = int(math.floor((self.maxy - maxy) / size))
        row1 = int(math.ceil((self.maxy - miny) / size)) - 1
        return (
            max(col0, 0),
            max(row0, 0),
            min(max(col1, col0), last),
            min(max(row1, row0), last),
        )

    def transform(self, level: int, col0: int, row0: int) -> Affine:
        """Affine transform of a raster whose top-left pixel is tile ``(col0, row0)``'s."""
        m = self.pixel_m(level)
        return Affine(
            m,
            0.0,
            self.minx + col0 * TILE_PX * m,
            0.0,
            -m,
            self.maxy - row0 * TILE_PX * m,
        )

    def to_pixel(
        self, level: int, x: np.ndarray | float, y: np.ndarray | float
    ) -> tuple[np.ndarray, np.ndarray]:
        """Continuous level pixel coordinates (0 = west / north edge) of EPSG:3035 points."""
        m = self.pixel_m(level)
        return (np.asarray(x) - self.minx) / m, (self.maxy - np.asarray(y)) / m


def tile_path(map_dir: Path, level: int, col: int, row: int) -> Path:
    """Path of a tile (E0 = the versioned ``data/map/height`` tiles)."""
    if level == 0:
        return map_dir / E0_DIR_NAME / f"h_{col}_{row}.png"
    return map_dir / PYRAMID_DIR_NAME / f"E{level}" / f"{col}_{row}.png"


@dataclass
class Cluster:
    """Zones whose tile rectangles overlap at one level, baked together."""

    level: int
    zones: list[Zone]
    col0: int
    row0: int
    col1: int
    row1: int
    tiles: set[tuple[int, int]]

    @property
    def key(self) -> str:
        """Stable identifier (level and zone ids)."""
        return f"E{self.level}_" + "+".join(sorted(z.id for z in self.zones))

    @property
    def shape(self) -> tuple[int, int]:
        """Raster shape (rows, cols)."""
        return (
            (self.row1 - self.row0 + 1) * TILE_PX,
            (self.col1 - self.col0 + 1) * TILE_PX,
        )


def zone_tiles(grid: PyramidGrid, zone: Zone, level: int) -> set[tuple[int, int]]:
    """Tiles ``(col, row)`` meeting the zone footprint at ``level``."""
    box = footprint(zone, level)
    if box is None:
        return set()
    col0, row0, col1, row1 = grid.tile_range(level, box)
    return {(c, r) for c in range(col0, col1 + 1) for r in range(row0, row1 + 1)}


def _rect(tiles: set[tuple[int, int]]) -> tuple[int, int, int, int]:
    cols = [c for c, _ in tiles]
    rows = [r for _, r in tiles]
    return min(cols), min(rows), max(cols), max(rows)


def _overlap(a: tuple[int, int, int, int], b: tuple[int, int, int, int]) -> bool:
    return a[0] <= b[2] and b[0] <= a[2] and a[1] <= b[3] and b[1] <= a[3]


def clusters(grid: PyramidGrid, zones: list[Zone], level: int) -> list[Cluster]:
    """Group the zones whose tile rectangles overlap at ``level`` (until stable)."""
    groups: list[tuple[list[Zone], set[tuple[int, int]]]] = []
    for zone in zones:
        tiles = zone_tiles(grid, zone, level)
        if tiles:
            groups.append(([zone], tiles))
    merged = True
    while merged:
        merged = False
        for i in range(len(groups)):
            for j in range(i + 1, len(groups)):
                if _overlap(_rect(groups[i][1]), _rect(groups[j][1])):
                    groups[i] = (
                        groups[i][0] + groups[j][0],
                        groups[i][1] | groups[j][1],
                    )
                    del groups[j]
                    merged = True
                    break
            if merged:
                break
    result = [
        Cluster(level, members, *_rect(tiles), tiles) for members, tiles in groups
    ]
    result.sort(key=lambda c: c.key)
    return result


# ------------------------------------------------------------------------ sampling


def linear_axis(
    n_out: int, factor: float, offset: float, n_in: int
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Indices and weights of a linear resampling along one axis.

    Output pixel ``i`` (centre ``i + 0.5``) maps to input coordinate
    ``(i + 0.5) / factor + offset`` (input pixel ``j`` has centre ``j + 0.5``).
    """
    coord = (np.arange(n_out, dtype=np.float64) + 0.5) / factor + offset - 0.5
    j0 = np.floor(coord).astype(np.int64)
    t = (coord - j0).astype(np.float32)
    j0c = np.clip(j0, 0, n_in - 1)
    j1c = np.clip(j0 + 1, 0, n_in - 1)
    return j0c, j1c, t


def bilinear(
    source: np.ndarray,
    shape: tuple[int, int],
    factor: float,
    offset_rows: float,
    offset_cols: float,
) -> np.ndarray:
    """Separable bilinear sampling of ``source`` on a grid ``factor`` times finer.

    NaN in ``source`` propagates to the output pixels that touch it.
    """
    r0, r1, tr = linear_axis(shape[0], factor, offset_rows, source.shape[0])
    c0, c1, tc = linear_axis(shape[1], factor, offset_cols, source.shape[1])
    rows = source[r0] * (1.0 - tr)[:, None] + source[r1] * tr[:, None]
    return (rows[:, c0] * (1.0 - tc)[None, :] + rows[:, c1] * tc[None, :]).astype(
        np.float32
    )


def read_tile_m(path: Path) -> np.ndarray | None:
    """Heights (m) of a tile, ``None`` when missing or unreadable (being written)."""
    if not path.exists():
        return None
    try:
        return terrain.uint16_to_height(terrain.read_png16(path)).astype(np.float32)
    except (OSError, ValueError):
        return None


def ancestor_heights(
    grid: PyramidGrid,
    level: int,
    col0: int,
    row0: int,
    shape: tuple[int, int],
    map_dir: Path = MAP_DIR,
) -> np.ndarray:
    """What the engine shows without level ``level``: finest coarser tile, bilinear.

    Returns the heights (m) on the raster of ``shape`` whose top-left pixel is
    tile ``(col0, row0)``'s at ``level``.
    """
    result = np.full(shape, np.nan, dtype=np.float32)
    for coarse in range(level - 1, -1, -1):
        need = np.isnan(result)
        if not need.any():
            break
        factor = 2 ** (level - coarse)
        # Coarse pixel range covering the raster, plus one pixel for bilinear.
        p0 = (col0 * TILE_PX) / factor
        q0 = (row0 * TILE_PX) / factor
        p1 = p0 + shape[1] / factor
        q1 = q0 + shape[0] / factor
        tc0, tr0 = int((p0 - 1) // TILE_PX), int((q0 - 1) // TILE_PX)
        tc1, tr1 = int((p1 + 1) // TILE_PX), int((q1 + 1) // TILE_PX)
        last = grid.tiles_per_side(coarse) - 1
        tc0, tr0 = max(tc0, 0), max(tr0, 0)
        tc1, tr1 = min(tc1, last), min(tr1, last)
        if tc0 > tc1 or tr0 > tr1:
            continue
        mosaic = np.full(
            ((tr1 - tr0 + 1) * TILE_PX, (tc1 - tc0 + 1) * TILE_PX), np.nan, np.float32
        )
        found = False
        for tr in range(tr0, tr1 + 1):
            for tc in range(tc0, tc1 + 1):
                tile = read_tile_m(tile_path(map_dir, coarse, tc, tr))
                if tile is None:
                    continue
                found = True
                mosaic[
                    (tr - tr0) * TILE_PX : (tr - tr0 + 1) * TILE_PX,
                    (tc - tc0) * TILE_PX : (tc - tc0 + 1) * TILE_PX,
                ] = tile
        if not found:
            continue
        sampled = bilinear(
            mosaic, shape, factor, q0 - tr0 * TILE_PX, p0 - tc0 * TILE_PX
        )
        result[need] = sampled[need]
    return result


# -------------------------------------------------------------------------- source


def source_heights(
    cluster: Cluster,
    grid: PyramidGrid,
    chunks: dict[str, list[tuple[str, Path]]],
) -> np.ndarray:
    """Mosaic of the zone sources on the cluster raster (NaN where no data).

    ``chunks`` maps a zone id to its ``(source key, chunk path)`` list in
    priority order; the first valid value wins.
    """
    shape = cluster.shape
    transform = grid.transform(cluster.level, cluster.col0, cluster.row0)
    result = np.full(shape, np.nan, dtype=np.float32)
    for zone in cluster.zones:
        for source_key, path in chunks.get(zone.id, []):
            source = SOURCES[source_key]
            with rasterio.open(path) as dataset:
                data = detail_sources.clean_heights(dataset.read(1), dataset.nodata)
                src_transform = dataset.transform
                src_crs = dataset.crs
                bounds = dataset.bounds
            data += np.float32(source.datum_offset_m)
            if source_key == "glo30":
                data = dsm_to_ground(data)
            window = _dst_window(bounds, str(src_crs), transform, shape)
            if window is None:
                continue
            r0, r1, c0, c1 = window
            part = np.full((r1 - r0, c1 - c0), np.nan, dtype=np.float32)
            reproject(
                source=data,
                destination=part,
                src_transform=src_transform,
                src_crs=src_crs,
                src_nodata=np.nan,
                dst_transform=transform * Affine.translation(c0, r0),
                dst_crs="EPSG:3035",
                dst_nodata=np.nan,
                resampling=Resampling.average,
            )
            block = result[r0:r1, c0:c1]
            fill = np.isnan(block) & np.isfinite(part)
            block[fill] = part[fill]
    return result


def dsm_to_ground(data: np.ndarray, size_px: int = GLO30_OPENING_PX) -> np.ndarray:
    """Rough bare earth from the GLO-30 surface model (fallback zones only).

    A grey opening removes bumps narrower than ``size_px`` pixels (buildings,
    copses, hedgerows at 30 m), then a light blur hides the 30 m grid.
    """
    valid = np.isfinite(data)
    if not valid.any():
        return data
    filled = np.where(valid, data, np.nanmedian(data)).astype(np.float32)
    opened = ndimage.grey_opening(filled, size=(size_px, size_px))
    smooth = ndimage.gaussian_filter(opened, 1.0)
    return np.where(valid, smooth, np.nan).astype(np.float32)


def _dst_window(
    bounds: rasterio.coords.BoundingBox,
    crs: str,
    transform: Affine,
    shape: tuple[int, int],
) -> tuple[int, int, int, int] | None:
    """Rows/cols window of the destination raster covered by a source box."""
    xs = np.linspace(bounds.left, bounds.right, 11)
    ys = np.linspace(bounds.bottom, bounds.top, 11)
    gx, gy = np.meshgrid(xs, ys)
    x, y = Transformer.from_crs(crs, "EPSG:3035", always_xy=True).transform(
        gx.ravel(), gy.ravel()
    )
    inverse = ~transform
    cols, rows = inverse * (np.asarray(x), np.asarray(y))
    c0 = max(int(np.floor(cols.min())) - 2, 0)
    c1 = min(int(np.ceil(cols.max())) + 2, shape[1])
    r0 = max(int(np.floor(rows.min())) - 2, 0)
    r1 = min(int(np.ceil(rows.max())) + 2, shape[0])
    if c0 >= c1 or r0 >= r1:
        return None
    return r0, r1, c0, c1


def cluster_bbox_lonlat(
    cluster_bbox: tuple[float, float, float, float],
) -> tuple[float, float, float, float]:
    """Lon/lat box of an EPSG:3035 box."""
    minx, miny, maxx, maxy = cluster_bbox
    xs = np.linspace(minx, maxx, 11)
    ys = np.linspace(miny, maxy, 11)
    gx, gy = np.meshgrid(xs, ys)
    lon, lat = _TO_GEO.transform(gx.ravel(), gy.ravel())
    return float(lon.min()), float(lat.min()), float(lon.max()), float(lat.max())


# --------------------------------------------------------------------------- boost


def boost_base(
    grid: PyramidGrid,
    cluster: Cluster,
    fine: np.ndarray,
) -> np.ndarray:
    """σ 5 km base of the render boost on the cluster raster.

    Built at 90 m (level 2) from the cluster's own fine source, extended
    outward by nearest-neighbour fill and blurred, then sampled bilinearly.

    ZG3b fix: the footprint of a detail zone at E6-E7 (3-6 km half-size) is
    much smaller than 3σ (15 km, :data:`BASE_MARGIN_M`), so a base built by
    overlaying the fine source onto GLO-90 *before* blurring barely differs
    from unblended GLO-90 at the footprint's centre -- the blur draws almost
    entirely on the surrounding margin. GLO-90 is a surface model (biased
    high by buildings in cities) and, being genuinely regional, also carries
    real relief several km away (hills around a river valley); either one
    pulled into the base pushes ``local = fine - base`` deeply negative and
    :func:`apply_boost` then digs real, positive-elevation land below sea
    level (observed: Southwark/Bermondsey/Lambeth/Kennington at -11 to -15 m
    against a real +2-5 m ODN, the City at 4.9-7.6 m against a real ~15 m).
    Extending the fine source itself outward (instead of leaking in GLO-90)
    keeps the base representative of the zone's own relief. GLO-90 is only
    used where the whole raster carries no fine data at all (an isolated
    cluster at the pyramid's edge).
    """
    level = cluster.level
    factor = 2 ** (level - BASE_LEVEL)
    base_m = grid.pixel_m(BASE_LEVEL)
    margin = int(math.ceil(BASE_MARGIN_M / base_m))
    cols = cluster.shape[1] // factor + 2 * margin
    rows = cluster.shape[0] // factor + 2 * margin
    fine_coarse = detail_sources.block_reduce_mean(fine, factor)
    extended = np.full((rows, cols), np.nan, dtype=np.float32)
    extended[
        margin : margin + fine_coarse.shape[0], margin : margin + fine_coarse.shape[1]
    ] = fine_coarse
    valid = np.isfinite(extended)
    if valid.any():
        indices = ndimage.distance_transform_edt(
            ~valid, return_distances=False, return_indices=True
        )
        filled = extended[tuple(indices)]
    else:
        c0 = cluster.col0 * TILE_PX // factor - margin
        r0 = cluster.row0 * TILE_PX // factor - margin
        size = grid.tiles_per_side(BASE_LEVEL) * TILE_PX
        map_grid = MapGrid(
            (
                grid.minx,
                grid.maxy - size * base_m,
                grid.minx + size * base_m,
                grid.maxy,
            ),
            size,
        )
        names = [p.stem for p in copernicus.RAW_DIR.glob("Copernicus_DSM_COG_30_*.tif")]
        coarse = copernicus.resample_to_grid(map_grid, (c0, r0, cols, rows), names)
        filled = np.where(np.isfinite(coarse), coarse, 0.0).astype(np.float32)
    blurred = ndimage.gaussian_filter(filled, BOOST_SIGMA_M / base_m)
    return bilinear(blurred, cluster.shape, factor, margin, margin)


def apply_boost(height: np.ndarray, base: np.ndarray) -> np.ndarray:
    """:func:`relief_shade.boost_relief` with an external base, on land (> 0 m).

    Land (source height above :data:`MIN_LAND_M`) never drops below
    :func:`relief_shade.valley_floor` of its real height once boosted: the
    same monotone floor as E0-E4 (SZ2), which also keeps it above sea level
    (ZG3b) and low banks a few metres above the water (ZG7a), whatever the
    base (real hills a few km off, a GLO-90 fallback at a data gap).
    """
    local = np.clip(height - base, -BOOST_LIMIT_M, BOOST_LIMIT_M)
    t = np.clip(
        (base - BOOST_FADE_M[0]) / (BOOST_FADE_M[1] - BOOST_FADE_M[0]), 0.0, 1.0
    )
    fade = 1.0 - t * t * (3.0 - 2.0 * t)
    boosted = floor_valleys(height + BOOST_GAIN * local * fade, height)
    land = height > MIN_LAND_M
    return np.where(land, boosted, height).astype(np.float32)


# -------------------------------------------------------------------------- blend


def footprint_weight(grid: PyramidGrid, cluster: Cluster) -> np.ndarray:
    """Max over the zones of the smooth ramp 0 (footprint edge) → 1 (inside)."""
    level = cluster.level
    rows, cols = cluster.shape
    weight = np.zeros(cluster.shape, dtype=np.float32)
    col_centres = cluster.col0 * TILE_PX + np.arange(cols, dtype=np.float64) + 0.5
    row_centres = cluster.row0 * TILE_PX + np.arange(rows, dtype=np.float64) + 0.5
    for zone in cluster.zones:
        box = footprint(zone, level)
        if box is None:
            continue
        px0, py1 = grid.to_pixel(level, box[0], box[1])
        px1, py0 = grid.to_pixel(level, box[2], box[3])
        band = FADE_FRACTION * (px1 - px0) / 2.0
        dx = np.minimum(col_centres - px0, px1 - col_centres) / band
        dy = np.minimum(row_centres - py0, py1 - row_centres) / band
        tx = np.clip(dx, 0.0, 1.0)
        ty = np.clip(dy, 0.0, 1.0)
        sx = tx * tx * (3.0 - 2.0 * tx)
        sy = ty * ty * (3.0 - 2.0 * ty)
        weight = np.maximum(weight, (sy[:, None] * sx[None, :]).astype(np.float32))
    return weight


def validity_weight(valid: np.ndarray, fade_px: int = COAST_FADE_PX) -> np.ndarray:
    """0 outside ``valid``, rising to 1 over ``fade_px`` pixels inside it."""
    if valid.all():
        return np.ones(valid.shape, dtype=np.float32)
    distance = ndimage.distance_transform_edt(valid)
    return np.clip(distance / (fade_px + 1.0), 0.0, 1.0).astype(np.float32)


def blend(fine: np.ndarray, ancestor: np.ndarray, weight: np.ndarray) -> np.ndarray:
    """``w * fine + (1 - w) * ancestor`` (the ancestor where fine is NaN)."""
    fine_filled = np.where(np.isfinite(fine), fine, ancestor)
    result = weight * fine_filled + (1.0 - weight) * ancestor
    return np.where(np.isfinite(result), result, fine_filled).astype(np.float32)


# --------------------------------------------------------------------------- bake


@dataclass
class ClusterResult:
    """Output of :func:`bake_cluster` (run in a worker process)."""

    key: str
    tiles: list[tuple[int, int]]
    bytes_written: int
    parent_updates: str | None  # .npz path with the 2 x 2 means for the parents
    erased_fraction: float
    valid_fraction: float
    seconds: float


def bake_cluster(
    cluster: Cluster,
    grid: PyramidGrid,
    chunks: dict[str, list[tuple[str, Path]]],
    osm_paths: dict[str, Path],
    map_dir: Path = MAP_DIR,
    preview_zone: str | None = None,
) -> ClusterResult:
    """Bake one cluster at its level and write its tiles."""
    started = time.perf_counter()
    level = cluster.level
    pixel_m = grid.pixel_m(level)
    transform = grid.transform(level, cluster.col0, cluster.row0)
    ancestor = ancestor_heights(
        grid, level, cluster.col0, cluster.row0, cluster.shape, map_dir
    )
    fine = source_heights(cluster, grid, chunks)
    fine = anachronisms.fill_small_holes(fine, int(HOLE_MAX_M2 / pixel_m**2))
    shapes = []
    for zone in cluster.zones:
        path = osm_paths.get(zone.id)
        if path is not None and path.exists():
            shapes.extend(
                anachronisms.osm_shapes(json.loads(path.read_text(encoding="utf-8")))
            )
    mask = anachronisms.modern_mask(shapes, transform, cluster.shape)
    mask &= np.isfinite(fine)
    before = fine.copy() if preview_zone else None
    if mask.any():
        fine = anachronisms.laplace_fill(fine, mask)
    if preview_zone and before is not None:
        write_preview(preview_zone, before, fine, mask, pixel_m)
    # Sea and foreshore values of the source (below MIN_LAND_M where the ancestor
    # is sea) give way to the ancestor's bathymetry: no chunk-shaped steps offshore.
    with np.errstate(invalid="ignore"):
        offshore = (fine < MIN_LAND_M) & ~(ancestor > 0.0)
    fine[offshore] = np.nan
    valid = np.isfinite(fine)
    base = boost_base(grid, cluster, fine)
    boosted = apply_boost(np.where(valid, fine, 0.0).astype(np.float32), base)
    boosted[~valid] = np.nan
    weight = footprint_weight(grid, cluster) * validity_weight(valid)
    baked = blend(boosted, ancestor, weight)
    encoded = terrain.height_to_uint16(baked)
    written = 0
    tiles = sorted(cluster.tiles)
    for col, row in tiles:
        r = (row - cluster.row0) * TILE_PX
        c = (col - cluster.col0) * TILE_PX
        path = tile_path(map_dir, level, col, row)
        path.parent.mkdir(parents=True, exist_ok=True)
        partial = path.with_suffix(".part.png")
        terrain.write_png16(encoded[r : r + TILE_PX, c : c + TILE_PX], partial)
        partial.replace(path)
        written += path.stat().st_size
    parent_file = _parent_updates(cluster, baked, weight)
    inside = footprint_weight(grid, cluster) > 0.999
    return ClusterResult(
        key=cluster.key,
        tiles=tiles,
        bytes_written=written,
        parent_updates=str(parent_file) if parent_file else None,
        erased_fraction=float(mask[inside].mean()) if inside.any() else 0.0,
        valid_fraction=float(valid[inside].mean()) if inside.any() else 0.0,
        seconds=time.perf_counter() - started,
    )


def _parent_updates(
    cluster: Cluster, baked: np.ndarray, weight: np.ndarray
) -> Path | None:
    """Save the 2 x 2 means (and mean weights) of the baked raster for the parents."""
    if cluster.level - 1 < LEVELS[0]:
        return None
    means = baked.reshape(baked.shape[0] // 2, 2, baked.shape[1] // 2, 2).mean(
        axis=(1, 3)
    )
    weights = weight.reshape(weight.shape[0] // 2, 2, weight.shape[1] // 2, 2).mean(
        axis=(1, 3)
    )
    path = DETAIL_RAW_DIR / "_work" / f"{cluster.key}_parents.npz"
    path.parent.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(
        path,
        means=means.astype(np.float32),
        weights=weights.astype(np.float32),
        origin=np.array([cluster.col0, cluster.row0, cluster.level]),
        tiles=np.array(sorted(cluster.tiles)),
    )
    return path


def apply_parent_updates(paths: list[str], map_dir: Path = MAP_DIR) -> int:
    """Blend the 2 x 2 child means into the parent tiles (serial, after a level)."""
    touched = 0
    pending: dict[tuple[int, int, int], np.ndarray] = {}
    for name in paths:
        data = np.load(name)
        means, weights = data["means"], data["weights"]
        col0, row0, level = (int(v) for v in data["origin"])
        parent_level = level - 1
        half = TILE_PX // 2
        for col, row in (tuple(int(v) for v in t) for t in data["tiles"]):
            key = (parent_level, col // 2, row // 2)
            if key not in pending:
                tile = read_tile_m(tile_path(map_dir, *key))
                if tile is None:
                    continue
                pending[key] = tile
            r = (row - row0) * half
            c = (col - col0) * half
            pr = (row % 2) * half
            pc = (col % 2) * half
            block = pending[key][pr : pr + half, pc : pc + half]
            w = weights[r : r + half, c : c + half]
            block[...] = w * means[r : r + half, c : c + half] + (1.0 - w) * block
    for (level, col, row), tile in pending.items():
        path = tile_path(map_dir, level, col, row)
        partial = path.with_suffix(".part.png")
        terrain.write_png16(terrain.height_to_uint16(tile), partial)
        partial.replace(path)
        touched += 1
    return touched


# ------------------------------------------------------------------------ preview


def hillshade(
    height: np.ndarray, pixel_m: float, azimuth: float = 315.0, altitude: float = 40.0
) -> np.ndarray:
    """8-bit hillshade (vertical exaggeration x2) with NaN in dark blue-grey."""
    h = np.where(
        np.isfinite(height),
        height,
        np.nanmin(height) if np.isfinite(height).any() else 0.0,
    )
    gy, gx = np.gradient(h * 2.0, pixel_m)
    slope = np.arctan(np.hypot(gx, gy))
    aspect = np.arctan2(-gx, gy)
    az = math.radians(360.0 - azimuth + 90.0)
    alt = math.radians(altitude)
    shade = np.sin(alt) * np.cos(slope) + np.cos(alt) * np.sin(slope) * np.cos(
        az - aspect
    )
    shade = np.clip(shade * 255.0, 0, 255).astype(np.uint8)
    shade[~np.isfinite(height)] = 40
    return shade


def write_preview(
    zone_id: str,
    before: np.ndarray,
    after: np.ndarray,
    mask: np.ndarray,
    pixel_m: float,
    size: int = 720,
) -> Path:
    """Side-by-side hillshade before / after erasing, mask outlined in red."""
    from PIL import Image

    rows, cols = before.shape
    side = min(rows, cols)
    r0, c0 = (rows - side) // 2, (cols - side) // 2
    window = (slice(r0, r0 + side), slice(c0, c0 + side))
    left = hillshade(before[window], pixel_m)
    right = hillshade(after[window], pixel_m)
    edge = mask[window] & ~ndimage.binary_erosion(mask[window], iterations=2)
    left_rgb = np.stack([left] * 3, axis=-1)
    left_rgb[edge] = (220, 40, 40)
    right_rgb = np.stack([right] * 3, axis=-1)
    image = np.concatenate(
        [left_rgb, np.full((side, max(side // 100, 4), 3), 255, np.uint8), right_rgb],
        axis=1,
    )
    picture = Image.fromarray(image)
    scale = size / side
    picture = picture.resize(
        (int(picture.width * scale), size), Image.Resampling.LANCZOS
    )
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    path = PREVIEW_DIR / f"{zone_id}_avant_apres.png"
    picture.save(path, optimize=True)
    return path


# ------------------------------------------------------------------------ manifest


def tiles_rle(tiles: set[tuple[int, int]]) -> list[dict]:
    """Run-length encoding of ``(col, row)`` tiles: rows of ``[col_start, length]`` runs."""
    by_row: dict[int, list[int]] = {}
    for col, row in tiles:
        by_row.setdefault(row, []).append(col)
    result = []
    for row in sorted(by_row):
        cols = sorted(by_row[row])
        runs: list[list[int]] = []
        for col in cols:
            if runs and runs[-1][0] + runs[-1][1] == col:
                runs[-1][1] += 1
            else:
                runs.append([col, 1])
        result.append({"row": row, "runs": runs})
    return result


def tiles_on_disk(map_dir: Path, level: int) -> set[tuple[int, int]]:
    """Tiles ``(col, row)`` present in ``pyramid/E<level>``."""
    folder = map_dir / PYRAMID_DIR_NAME / f"E{level}"
    tiles = set()
    for path in folder.glob("*.png"):
        match = re.fullmatch(r"(\d+)_(\d+)", path.stem)
        if match:
            tiles.add((int(match[1]), int(match[2])))
    return tiles


LEVEL_SOURCE = (
    "MNT nationaux sans sursol (IGN RGE ALTI, EA LiDAR, AHN, DHM Vlaanderen ; repli "
    "GLO-30), anachronismes effacés (OSM) ; zones de data/map/detail_zones.json"
)


def level_line(
    level: int, grid: PyramidGrid, tiles: set[tuple[int, int]], bbox: list[float] | None
) -> str:
    """One compact manifest entry ``{ "level": k, ... }`` (single line)."""
    entry: dict = {
        "level": level,
        "meters_per_px": round(grid.pixel_m(level), 4),
        "tier": 3,
        "source": LEVEL_SOURCE,
    }
    if bbox is not None:
        entry["bbox_lonlat"] = [round(v, 4) for v in bbox]
    entry["tiles_rle"] = tiles_rle(tiles)
    body = json.dumps(entry, ensure_ascii=False, separators=(", ", ": "))
    return "{ " + body[1:-1] + " }"


def update_manifest(lines: dict[int, str], path: Path = MANIFEST_FILE) -> None:
    """Replace only the ``levels`` lines of the given levels (others kept byte for byte)."""
    text = path.read_text(encoding="utf-8")
    out = []
    pattern = re.compile(r'^(\s*)\{\s*"level":\s*(\d+)\s*,.*?\}(,?)\s*$')
    for line in text.split("\n"):
        match = pattern.match(line)
        if match and int(match[2]) in lines:
            out.append(f"{match[1]}{lines[int(match[2])]}{match[3]}")
        else:
            out.append(line)
    new_text = "\n".join(out)
    json.loads(new_text)  # still valid JSON
    path.write_text(new_text, encoding="utf-8")


# --------------------------------------------------------------------------- build


@dataclass
class DetailResult:
    """Summary of :func:`build`."""

    tiles: dict[int, int]
    bytes_by_level: dict[int, int]
    raw_bytes: int
    zones: list[str]
    seconds: float
    notes: list[str] = field(default_factory=list)


def params_hash(zone: Zone, level: int) -> str:
    """Hash of everything a zone's bake depends on (``done`` markers)."""
    payload = json.dumps(
        [BAKE_VERSION, zone.__dict__ | {"level_half_km": zone.level_half_km}, level],
        sort_keys=True,
        default=str,
    )
    return hashlib.sha1(payload.encode()).hexdigest()[:16]


def _done_path(zone: Zone, level: int) -> Path:
    return DETAIL_RAW_DIR / zone.id / f"done_E{level}.json"


def cluster_done(cluster: Cluster, map_dir: Path, since: float | None = None) -> bool:
    """Whether every zone of the cluster has a current marker and every tile exists.

    With ``since`` (a forced or stale bake in progress, :mod:`bake_stamp`),
    markers written before it do not count.
    """
    for zone in cluster.zones:
        marker = _done_path(zone, cluster.level)
        if bake_stamp.needs_rebake(marker, since):
            return False
        if json.loads(marker.read_text()).get("hash") != params_hash(
            zone, cluster.level
        ):
            return False
    return all(
        tile_path(map_dir, cluster.level, c, r).exists() for c, r in cluster.tiles
    )


def fetch_all(
    zones: list[Zone], grid: PyramidGrid, notes: list[str]
) -> dict[int, dict[str, list[tuple[str, Path]]]]:
    """Fetch every chunk of every zone and level (threads; one request per host at a time)."""
    result: dict[int, dict[str, list[tuple[str, Path]]]] = {lvl: {} for lvl in LEVELS}
    with ThreadPoolExecutor(8) as pool:
        futures = []
        for zone in zones:
            for level in LEVELS:
                box = footprint(zone, level)
                if box is None:
                    continue
                for key in zone.sources():
                    futures.append(
                        (
                            zone,
                            level,
                            key,
                            pool.submit(
                                detail_sources.fetch_zone_level,
                                key,
                                zone.id,
                                level,
                                box,
                                grid.pixel_m(level),
                            ),
                        )
                    )
        for zone, level, key, future in futures:
            try:
                paths = future.result()
            except detail_sources.FetchError as error:
                notes.append(
                    f"{zone.id} E{level} {key}: échec ({error}) ; repli GLO-30"
                )
                paths = []
                key = "glo30"
                box = footprint(zone, level)
                paths = detail_sources.glo30_tiles(box) if box else []
            result[level].setdefault(zone.id, []).extend((key, p) for p in paths)
    return result


def fetch_osm_all(zones: list[Zone], notes: list[str], force: bool) -> dict[str, Path]:
    """Overpass answers of every zone (serial: the public instance allows 2 slots)."""
    paths = {}
    for zone in zones:
        box = footprint(zone, LEVELS[0])
        if box is None:
            continue
        try:
            paths[zone.id] = anachronisms.fetch_osm(
                zone.id,
                cluster_bbox_lonlat(box),
                canals=zone.kind != "city",
                force=force,
            )
        except RuntimeError as error:
            notes.append(f"{zone.id} : OSM indisponible ({error}), pas d'effacement")
    return paths


def raw_bytes() -> int:
    """Size of the raw detail cache."""
    return sum(p.stat().st_size for p in DETAIL_RAW_DIR.rglob("*") if p.is_file())


def build(
    zone_ids: tuple[str, ...] = (),
    force: bool = False,
    map_dir: Path = MAP_DIR,
    workers: int = WORKERS,
    log=print,  # noqa: ANN001
) -> DetailResult:
    """Bake E5-E7 for the given zones (all when empty) and update the manifest."""
    started = time.perf_counter()
    grid = PyramidGrid.from_map(map_dir)
    all_zones = load_zones()
    wanted = set(zone_ids) or {z.id for z in all_zones}
    unknown = wanted - {z.id for z in all_zones}
    if unknown:
        raise ValueError(f"zones inconnues : {sorted(unknown)}")
    notes: list[str] = []
    zones = [z for z in all_zones if z.id in wanted]
    whole = wanted == {z.id for z in all_zones}
    pyramid_dir = map_dir / PYRAMID_DIR_NAME
    # A forced (or stale) bake of every zone is stamped and resumable; a bake of
    # some zones only rewrites them with ``force``.
    if whole:
        since = bake_stamp.begin(pyramid_dir, "tier3", BAKE_VERSION, force)
    else:
        since = time.time() if force else None
    log(f"Téléchargement des MNT : {len(zones)} zones")
    chunks = fetch_all(zones, grid, notes)
    log("Téléchargement OSM (anachronismes)")
    osm = fetch_osm_all(zones, notes, force=False)
    for level in LEVELS:
        # Clusters use every zone so that neighbours of a re-baked zone stay coherent.
        level_clusters = [
            c
            for c in clusters(grid, all_zones, level)
            if any(z.id in wanted for z in c.zones)
        ]
        todo = [c for c in level_clusters if not cluster_done(c, map_dir, since)]
        missing = {z.id for c in todo for z in c.zones} - {z.id for z in zones}
        if missing:
            extra = [z for z in all_zones if z.id in missing]
            for lvl, items in fetch_all(extra, grid, notes).items():
                for zid, paths in items.items():
                    chunks[lvl].setdefault(zid, paths)
            osm.update(fetch_osm_all(extra, notes, force=False))
        log(f"E{level} : {len(todo)} grappes à cuire sur {len(level_clusters)}")
        parent_files = []
        with ProcessPoolExecutor(max(1, min(workers, len(todo) or 1))) as pool:
            futures = {
                pool.submit(
                    bake_cluster,
                    cluster,
                    grid,
                    {z.id: chunks[level].get(z.id, []) for z in cluster.zones},
                    osm,
                    map_dir,
                    _preview_for(cluster),
                ): cluster
                for cluster in todo
            }
            for future, cluster in futures.items():
                result = future.result()
                if result.parent_updates:
                    parent_files.append(result.parent_updates)
                for zone in cluster.zones:
                    marker = _done_path(zone, level)
                    marker.parent.mkdir(parents=True, exist_ok=True)
                    marker.write_text(
                        json.dumps(
                            {
                                "hash": params_hash(zone, level),
                                "tiles": len(result.tiles),
                                "bytes": result.bytes_written,
                                "erased_fraction": round(result.erased_fraction, 4),
                                "valid_fraction": round(result.valid_fraction, 4),
                            }
                        )
                    )
                log(
                    f"  {result.key} : {len(result.tiles)} tuiles, "
                    f"{result.bytes_written / 1e6:.1f} Mo, effacé {result.erased_fraction:.1%}, "
                    f"source {result.valid_fraction:.1%}, {result.seconds:.0f} s"
                )
        if parent_files:
            touched = apply_parent_updates(parent_files, map_dir)
            log(f"  E{level - 1} : {touched} tuiles parentes recalées (moyenne 2 x 2)")
    lines = {}
    counts, sizes = {}, {}
    for level in LEVELS:
        tiles = tiles_on_disk(map_dir, level)
        counts[level] = len(tiles)
        sizes[level] = sum(
            tile_path(map_dir, level, c, r).stat().st_size for c, r in tiles
        )
        lines[level] = level_line(level, grid, tiles, _zones_bbox(all_zones, level))
    update_manifest(lines)
    if whole:
        from cent_ans_tools.geo import pyramid

        bake_stamp.finish(pyramid_dir, "tier3", BAKE_VERSION, since)
        pyramid.set_manifest_bake_versions(map_dir, {"tier3": BAKE_VERSION})
    return DetailResult(
        tiles=counts,
        bytes_by_level=sizes,
        raw_bytes=raw_bytes(),
        zones=sorted(wanted),
        seconds=time.perf_counter() - started,
        notes=notes,
    )


def _preview_for(cluster: Cluster) -> str | None:
    """Zone id whose before/after preview this cluster writes (level 6 only)."""
    if cluster.level != 6:
        return None
    for zone in cluster.zones:
        if zone.id in PREVIEW_ZONES:
            return zone.id
    return None


def _zones_bbox(zones: list[Zone], level: int) -> list[float] | None:
    """Lon/lat box of every footprint at ``level``."""
    boxes = [cluster_bbox_lonlat(b) for z in zones if (b := footprint(z, level))]
    if not boxes:
        return None
    return [
        min(b[0] for b in boxes),
        min(b[1] for b in boxes),
        max(b[2] for b in boxes),
        max(b[3] for b in boxes),
    ]


# ---------------------------------------------------------------------- coast check


@dataclass
class LandGap:
    """E5-vs-E4 land gap of one zone (lot ZG3b): :func:`e5_e4_land_gap`."""

    zone_id: str
    n_pixels: int
    median_m: float | None
    p95_m: float | None
    max_m: float | None


#: Above this p95 gap (m), a zone is flagged (docstring of :func:`e5_e4_land_gap`).
LAND_GAP_ALERT_M = 5.0
#: Interior margin (of a footprint's half-size) excluded from the check: the
#: footprint edge blends into the E4 ancestor by construction (blend()), so a
#: gap there is expected and not a bake defect.
LAND_GAP_EDGE_WEIGHT = 0.98


def e5_e4_land_gap(zone: Zone, grid: PyramidGrid, map_dir: Path = MAP_DIR) -> LandGap:
    """Compare baked E5 land to the E4 ancestor over one zone's footprint.

    ``E5`` should read close to ``E4`` on land away from the footprint edge
    (E4 is itself only a coarser, boosted average of the same relief): a
    systematic gap flags a bake defect such as the ZG3b render-boost leak
    (real land baked far below the coarser levels, or below sea level). Water
    pixels (baked below :data:`MIN_LAND_M`) are excluded: E5-E7 legitimately
    show real river channels/foreshore the E4 ancestor cannot resolve.

    Returns ``LandGap`` with ``median_m/p95_m/max_m`` as ``None`` when the
    zone has no E5 tiles on disk (not yet baked).
    """
    level = 5
    box = footprint(zone, level)
    if box is None:
        return LandGap(zone.id, 0, None, None, None)
    col0, row0, col1, row1 = grid.tile_range(level, box)
    shape = ((row1 - row0 + 1) * TILE_PX, (col1 - col0 + 1) * TILE_PX)
    e5 = np.full(shape, np.nan, dtype=np.float32)
    any_tile = False
    for row in range(row0, row1 + 1):
        for col in range(col0, col1 + 1):
            tile = read_tile_m(tile_path(map_dir, level, col, row))
            if tile is None:
                continue
            any_tile = True
            r, c = (row - row0) * TILE_PX, (col - col0) * TILE_PX
            e5[r : r + TILE_PX, c : c + TILE_PX] = tile
    if not any_tile:
        return LandGap(zone.id, 0, None, None, None)
    ancestor = ancestor_heights(grid, level, col0, row0, shape, map_dir)
    single = Cluster(level, [zone], col0, row0, col1, row1, set())
    weight = footprint_weight(grid, single)
    land = np.isfinite(e5) & (e5 > MIN_LAND_M) & (weight >= LAND_GAP_EDGE_WEIGHT)
    land &= np.isfinite(ancestor)
    if not land.any():
        return LandGap(zone.id, 0, None, None, None)
    gap = np.abs(e5[land] - ancestor[land])
    return LandGap(
        zone.id,
        int(land.sum()),
        float(np.median(gap)),
        float(np.percentile(gap, 95)),
        float(np.max(gap)),
    )


def land_gap_report(
    zone_ids: tuple[str, ...] = (), map_dir: Path = MAP_DIR
) -> list[LandGap]:
    """:func:`e5_e4_land_gap` of every zone (or ``zone_ids``), sorted by p95 gap."""
    grid = PyramidGrid.from_map(map_dir)
    zones = load_zones()
    wanted = set(zone_ids) or {z.id for z in zones}
    results = [e5_e4_land_gap(z, grid, map_dir) for z in zones if z.id in wanted]
    results.sort(key=lambda r: (r.p95_m is None, -(r.p95_m or 0.0)))
    return results
