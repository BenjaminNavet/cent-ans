"""Real relief of the historical battlefields (lot EP7, ADR 0035).

For each map of ``data/battle_maps/<id>.json`` (Crécy, Poitiers, Agincourt),
the ``site`` block gives the centre of the field (lon, lat) and the compass
bearing of the attacker's advance (+z of the battle). This module:

* samples Copernicus DEM GLO-30 (``tools/geo/raw/copernicus30``, see
  :mod:`cent_ans_tools.geo.glo30`) on the field, rotated to the battle frame,
  every :data:`FIELD_STEP_M` metres; GLO-30 is a surface model, so the canopy
  of the woods (ESA WorldCover trees) and the modern buildings are taken off
  by a grey opening and a light blur, and writes the ``relief`` block of the
  map (decimetres above the lowest point);
* bakes a horizon tile ``hist_<id>`` (26 km at 100 m, same format as lot EP2,
  :func:`cent_ans_tools.geo.horizon.encode_tile`) centred on the field and
  rotated to the battle frame, so that the rendered horizon needs no rotation:
  tile "north" is the battle's -z, tile "east" its +x; the skyline azimuths
  are in the same frame;
* writes a preview (hillshade, woods of today, field outline) for the authors
  of the map (``docs/img/ep7/<id>_site.png``).

Battle frame: ``x`` from 0 to the width, ``z`` from 0 (attacker's edge) to the
depth; a field point maps to ``centre + east_unit * (x - w/2) + north_unit *
(d/2 - z)`` where ``north_unit`` points at ``bearing + 180`` and ``east_unit``
at ``bearing - 90`` (compass bearings).
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
import rasterio
from PIL import Image
from pyproj import Transformer
from scipy import ndimage

from cent_ans_tools.geo import glo30, horizon

REPO_DIR = Path(__file__).resolve().parents[3]
MAPS_DIR = REPO_DIR / "data" / "battle_maps"
PREVIEW_DIR = REPO_DIR / "docs" / "img" / "ep7"

#: Spacing of the field relief written into the map (m).
FIELD_STEP_M = 20.0
#: Grey opening removing copses, hedgerows and buildings from the surface model (m).
GROUND_OPENING_M = 90.0
#: Blur of the bare-earth estimate (grid cells of FIELD_STEP_M).
GROUND_BLUR_CELLS = 1.2

_to_map = Transformer.from_crs("EPSG:4326", "EPSG:3035", always_xy=True)
_to_geo = Transformer.from_crs("EPSG:3035", "EPSG:4326", always_xy=True)


@dataclass(frozen=True)
class Frame:
    """Battle frame of a site in EPSG:3035."""

    cx: float
    cy: float
    east: tuple[float, float]
    north: tuple[float, float]
    width: float
    depth: float

    @staticmethod
    def of(site: dict, field: dict) -> Frame:
        """Frame of a map's ``site`` and ``field`` blocks."""
        cx, cy = _to_map.transform(site["lon"], site["lat"])
        bearing = math.radians(site["bearing_deg"])
        north_b = bearing + math.pi
        east_b = bearing - math.pi / 2.0
        return Frame(
            float(cx),
            float(cy),
            (math.sin(east_b), math.cos(east_b)),
            (math.sin(north_b), math.cos(north_b)),
            float(field["width_m"]),
            float(field["depth_m"]),
        )

    def field_to_map(
        self, x: np.ndarray, z: np.ndarray
    ) -> tuple[np.ndarray, np.ndarray]:
        """EPSG:3035 coordinates of battle-field points."""
        e = np.asarray(x, dtype=np.float64) - self.width / 2.0
        n = self.depth / 2.0 - np.asarray(z, dtype=np.float64)
        return (
            self.cx + self.east[0] * e + self.north[0] * n,
            self.cy + self.east[1] * e + self.north[1] * n,
        )

    def local_to_map(
        self, east_m: np.ndarray, north_m: np.ndarray
    ) -> tuple[np.ndarray, np.ndarray]:
        """EPSG:3035 coordinates of tile-frame offsets from the field centre."""
        e = np.asarray(east_m, dtype=np.float64)
        n = np.asarray(north_m, dtype=np.float64)
        return (
            self.cx + self.east[0] * e + self.north[0] * n,
            self.cy + self.east[1] * e + self.north[1] * n,
        )


class LonLatRaster:
    """Bilinear sampler of a set of EPSG:4326 GeoTIFF tiles over a bounding box."""

    def __init__(self, paths: list[Path], bbox: tuple[float, float, float, float]):
        """Read the window ``bbox`` (lon/lat) of every tile into one mosaic."""
        from rasterio.merge import merge

        datasets = [rasterio.open(path) for path in paths]
        try:
            data, transform = merge(datasets, bounds=bbox, nodata=np.nan)
        finally:
            for dataset in datasets:
                dataset.close()
        self.data = data[0].astype(np.float32)
        self.transform = transform

    def sample(self, lon: np.ndarray, lat: np.ndarray, order: int = 1) -> np.ndarray:
        """Values at lon/lat (NaN outside)."""
        inv = ~self.transform
        col, row = inv * (np.asarray(lon), np.asarray(lat))
        return ndimage.map_coordinates(
            self.data,
            [np.asarray(row) - 0.5, np.asarray(col) - 0.5],
            order=order,
            mode="nearest",
            cval=np.nan,
        )


def _bbox(frame: Frame, half_m: float) -> tuple[float, float, float, float]:
    xs = np.array([-half_m, half_m, -half_m, half_m])
    ys = np.array([-half_m, -half_m, half_m, half_m])
    lon, lat = _to_geo.transform(frame.cx + xs, frame.cy + ys)
    pad = 0.01
    return (min(lon) - pad, min(lat) - pad, max(lon) + pad, max(lat) + pad)


def _glo30(bbox: tuple[float, float, float, float]) -> LonLatRaster:
    names = [
        glo30.glo30_tile_name(lon, lat)
        for lon in range(math.floor(bbox[0]), math.floor(bbox[2]) + 1)
        for lat in range(math.floor(bbox[1]), math.floor(bbox[3]) + 1)
    ]
    paths = [
        glo30.glo30_path(name) for name in names if glo30.glo30_path(name).exists()
    ]
    if not paths:
        raise FileNotFoundError(f"no GLO-30 tile for {bbox} (cent-ans geo fetch-glo30)")
    return LonLatRaster(paths, bbox)


def _trees(bbox: tuple[float, float, float, float]) -> LonLatRaster | None:
    names = glo30.worldcover_tiles_in_bbox(bbox)
    paths = [
        glo30.worldcover_path(n) for n in names if glo30.worldcover_path(n).exists()
    ]
    if not paths:
        return None
    datasets = [rasterio.open(path) for path in paths]
    try:
        from rasterio.merge import merge

        classes, transform = merge(datasets, bounds=bbox, nodata=0)
    finally:
        for dataset in datasets:
            dataset.close()
    raster = LonLatRaster.__new__(LonLatRaster)
    raster.data = (classes[0] == glo30.WC_TREES).astype(np.float32)
    raster.transform = transform
    return raster


def field_relief(site: dict, field: dict) -> tuple[dict, np.ndarray, np.ndarray]:
    """``relief`` block of a map, plus the bare-earth grid (m) and the trees grid."""
    frame = Frame.of(site, field)
    nx = int(math.ceil(frame.width / FIELD_STEP_M)) + 1
    nz = int(math.ceil(frame.depth / FIELD_STEP_M)) + 1
    margin = 6  # cells around the field so the opening sees the edges
    xs = (np.arange(-margin, nx + margin) * FIELD_STEP_M)[None, :]
    zs = (np.arange(-margin, nz + margin) * FIELD_STEP_M)[:, None]
    mx, my = frame.field_to_map(
        np.broadcast_to(xs, (zs.size, xs.size)), np.broadcast_to(zs, (zs.size, xs.size))
    )
    lon, lat = _to_geo.transform(mx, my)
    bbox = _bbox(frame, max(frame.width, frame.depth))
    dsm = _glo30(bbox).sample(lon, lat)
    trees_raster = _trees(bbox)
    trees = (
        np.zeros_like(dsm)
        if trees_raster is None
        else np.clip(trees_raster.sample(lon, lat), 0.0, 1.0)
    )
    size = max(3, int(round(GROUND_OPENING_M / FIELD_STEP_M)) | 1)
    filled = np.where(np.isfinite(dsm), dsm, np.nanmedian(dsm)).astype(np.float32)
    # Canopy first (the radar sees ~10 m of the crowns), then the opening.
    cover = ndimage.gaussian_filter(trees, 1.5)
    lowered = filled - 10.0 * cover
    ground = ndimage.grey_opening(lowered, size=(size, size))
    ground = ndimage.gaussian_filter(ground, GROUND_BLUR_CELLS)
    ground = ground[margin : margin + nz, margin : margin + nx]
    trees = trees[margin : margin + nz, margin : margin + nx]
    base = float(ground.min())
    heights_dm = np.rint((ground - base) * 10.0).astype(int)
    relief = {
        "step_m": FIELD_STEP_M,
        "nx": nx,
        "nz": nz,
        "base_m": round(base, 1),
        "vertical_scale": 1.0,
        "heights_dm": heights_dm.ravel().tolist(),
        "source": "Copernicus DEM GLO-30 (© DLR e.V. 2010-2014 et © Airbus Defence and Space GmbH 2014-2018, programme Copernicus), couvert des bois ESA WorldCover 2021 retiré, ouverture morphologique 90 m (cent-ans geo battle-site)",
    }
    return relief, ground, trees


def bake_site_tile(
    key: str, site: dict, field: dict, sources: horizon.Sources
) -> horizon.HorizonTile:
    """Horizon tile of a site, in the battle frame."""
    frame = Frame.of(site, field)
    half = horizon.TILE_SPAN_M / 2.0
    offsets = np.arange(horizon.TILE_N) * horizon.STEP_M - half
    east = offsets[None, :].repeat(horizon.TILE_N, 0)
    north = -offsets[:, None].repeat(horizon.TILE_N, 1)
    mx, my = frame.local_to_map(east, north)
    lon, lat = _to_geo.transform(mx, my)
    bbox = _bbox(frame, half * 1.5)
    glo = _glo30(bbox)
    dsm = glo.sample(lon, lat)
    fine = sources.fine_height(mx, my)
    has = np.isfinite(dsm)
    # A 100 m tile: average the 30 m surface (3 x 3) and take the canopy off loosely.
    heights = np.where(
        has, ndimage.uniform_filter(np.nan_to_num(dsm, nan=0.0), 3), fine
    )
    coarse_sea = sources.open_sea(mx, my)
    sea = (has & (np.abs(np.nan_to_num(dsm)) <= 0.01) & (fine < 5.0)) | (
        ~has & coarse_sea & (fine <= 0.5)
    )
    heights = np.where(sea, np.minimum(heights, 0.0) - 3.0, heights).astype(np.float32)
    forest = np.clip(sources.forest_share(mx, my), 0.0, 1.0)
    classes = np.rint(forest * horizon.FOREST_MAX).astype(np.uint8)
    classes[sea] = horizon.SEA_CLASS
    centre = horizon.TILE_N // 2
    fx = int(frame.width / 2.0 / horizon.STEP_M)
    fz = int(frame.depth / 2.0 / horizon.STEP_M)
    footprint = heights[centre - fz : centre + fz + 1, centre - fx : centre + fx + 1]
    land = footprint[footprint > -1.0]
    ref = float(land.mean()) if land.size else 0.0

    def height_at(e: np.ndarray, n: np.ndarray) -> np.ndarray:
        x, y = frame.local_to_map(e, n)
        return sources.fine_height(x, y)

    def sea_at(e: np.ndarray, n: np.ndarray) -> np.ndarray:
        x, y = frame.local_to_map(e, n)
        return sources.open_sea(x, y) & (sources.fine_height(x, y) <= 0.5)

    angle, dist, share = horizon.skyline_profile(
        height_at, max(ref, 0.0) + horizon.EYE_M, sea_at=sea_at
    )
    return horizon.HorizonTile(
        province=key,
        lon=float(site["lon"]),
        lat=float(site["lat"]),
        ref_m=ref,
        heights_m=heights,
        classes=classes,
        skyline_deg=angle,
        skyline_dist_m=dist,
        sea_share=share,
        coast_bearing_deg=horizon.coast_bearing(classes),
    )


def write_preview(
    path: Path, ground: np.ndarray, trees: np.ndarray, step_m: float
) -> None:
    """Hillshade of the field (x right, z up: the attacker at the bottom), woods in green."""
    gy, gx = np.gradient(ground, step_m)
    # Light from the upper left of the image.
    shade = np.clip(0.6 - (gx * 0.7 - gy * 0.7) * 4.0, 0.0, 1.0)
    span = max(float(ground.max() - ground.min()), 1.0)
    level = (ground - ground.min()) / span
    rgb = (
        np.stack(
            [
                0.55 + 0.35 * level,
                0.52 + 0.30 * level,
                0.40 + 0.20 * level,
            ],
            axis=-1,
        )
        * shade[..., None]
    )
    woods = ndimage.gaussian_filter(trees, 0.8) > 0.4
    rgb[woods] = rgb[woods] * 0.45 + np.array([0.05, 0.25, 0.05]) * 0.55
    # Contours every 5 m.
    contour = np.abs(((ground - ground.min()) / 5.0) % 1.0 - 0.5) > 0.46
    rgb[contour] *= 0.7
    image = (np.clip(rgb, 0.0, 1.0) * 255).astype(np.uint8)[::-1]  # z = 0 at the bottom
    scale = 3
    Image.fromarray(image).resize(
        (image.shape[1] * scale, image.shape[0] * scale), Image.Resampling.NEAREST
    ).save(path)


def with_relief(text: str, relief: dict) -> str:
    """Map text with its ``relief`` block replaced (one line, the rest untouched).

    The block is written on a single line ``  "relief": {...},`` right before the
    top-level ``"weather"`` key, so that the hand-written map keeps its layout.
    """
    line = (
        '  "relief": '
        + json.dumps(relief, ensure_ascii=False, separators=(",", ":"))
        + ","
    )
    lines = [
        row for row in text.splitlines() if not row.lstrip().startswith('"relief": ')
    ]
    at = next(
        (i for i, row in enumerate(lines) if row.lstrip().startswith('"weather": ')),
        None,
    )
    if at is None:
        raise ValueError('the map has no "weather" line to put the relief before')
    lines.insert(at, line)
    return "\n".join(lines) + "\n"


def bake(map_ids: list[str] | None = None, tiles: bool = True) -> list[str]:
    """Bake the relief (and horizon tile) of every map, or of ``map_ids``."""
    done: list[str] = []
    sources = horizon.Sources() if tiles else None
    index_path = horizon.OUT_DIR / "index.json"
    index = json.loads(index_path.read_text(encoding="utf-8")) if tiles else {}
    PREVIEW_DIR.mkdir(parents=True, exist_ok=True)
    for path in sorted(MAPS_DIR.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        if "site" not in data or (map_ids and data["id"] not in map_ids):
            continue
        relief, ground, trees = field_relief(data["site"], data["field"])
        previous = data.get("relief", {})
        relief["vertical_scale"] = previous.get("vertical_scale", 1.0)
        path.write_text(
            with_relief(path.read_text(encoding="utf-8"), relief), encoding="utf-8"
        )
        write_preview(
            PREVIEW_DIR / f"{data['id']}_site.png", ground, trees, FIELD_STEP_M
        )
        if tiles and sources is not None:
            key = data.get("horizon") or f"hist_{data['id']}"
            tile = bake_site_tile(key, data["site"], data["field"], sources)
            blob = horizon.encode_tile(tile)
            (horizon.OUT_DIR / f"{key}.bin").write_bytes(blob)
            index.setdefault("sites", {})[key] = {
                "file": f"{key}.bin",
                "lon": round(tile.lon, 4),
                "lat": round(tile.lat, 4),
                "ref_m": round(tile.ref_m, 1),
                "terrain": data["terrain"],
                "coastal": False,
                "coast_bearing_deg": None
                if tile.coast_bearing_deg is None
                else round(tile.coast_bearing_deg, 1),
                "skyline_max_deg": round(float(tile.skyline_deg.max()), 2),
                "relief_max_m": round(float(tile.heights_m.max()), 0),
                "sea_share": round(
                    float((tile.classes == horizon.SEA_CLASS).mean()), 3
                ),
                "site": data["id"],
            }
        done.append(data["id"])
    if tiles:
        index["sites"] = dict(sorted(index["sites"].items()))
        index_path.write_text(
            json.dumps(index, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
        )
    return done
