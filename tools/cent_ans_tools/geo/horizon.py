"""Real relief around each province for the battle horizon (lot EP2, ADR 0032).

For every province a representative point is chosen (the polygon centroid, or,
for a coastal province, the land point about 5 km from the sea nearest to it) and
a square tile of :data:`TILE_SPAN_M` metres centred on it is baked:

* heights from Copernicus DEM GLO-90 (area average, the same cache as lot R1),
  falling back to the 8192² fine tiles (≈ 360 m, ETOPO outside the Copernicus
  box) where Copernicus has no data;
* a class byte per cell: forest share (0-200, from ``splat.png`` channel B) or
  :data:`SEA_CLASS` for the open sea;
* the real skyline seen from the point (eye :data:`EYE_M` above the ground):
  for :data:`PROFILE_COUNT` azimuths (0 = north, clockwise), the highest
  elevation angle between :data:`SKYLINE_NEAR_M` and :data:`SKYLINE_FAR_M` (earth
  curvature and standard refraction included), its distance, and the share of
  sea along the ray.

The grid is EPSG:3035 (north up, row 0 = north), one sample every
:data:`STEP_M` metres, sample ``(i, j)`` at ``centre + (-half + i * step,
half - j * step)``. Output: ``game/assets/horizon/relief/<province>.bin`` (see
:func:`encode_tile`) and ``index.json``. Rendering only: no game rule reads it.
"""

from __future__ import annotations

import json
import math
import struct
import time
import zlib
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from shapely.geometry import Point, shape

from cent_ans_tools.geo import copernicus, relief, terrain
from cent_ans_tools.geo.project import MapGrid

REPO_DIR = Path(__file__).resolve().parents[3]
MAP_DIR = REPO_DIR / "data" / "map"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"
OUT_DIR = REPO_DIR / "game" / "assets" / "horizon" / "relief"

MAGIC = b"HZR1"
TILE_SPAN_M = 26_000.0
STEP_M = 100.0
TILE_N = int(TILE_SPAN_M / STEP_M) + 1  # 261 samples, corners included
PROFILE_COUNT = 256
EYE_M = 25.0
SKYLINE_NEAR_M = 12_000.0
SKYLINE_FAR_M = 150_000.0
SEA_SHARE_REACH_M = 40_000.0
EARTH_RADIUS_M = 6_371_000.0
REFRACTION = 0.13
SEA_CLASS = 255
FOREST_MAX = 200
#: Coastal provinces: target distance to the sea of the representative point.
COAST_TARGET_PX = (3.0, 5.0)
#: Half extent (m) of the field footprint used for the reference altitude.
FIELD_HALF_M = (600.0, 400.0)


@dataclass
class HorizonTile:
    """One baked tile."""

    province: str
    lon: float
    lat: float
    ref_m: float
    heights_m: np.ndarray  # float32 (TILE_N, TILE_N), row 0 = north
    classes: np.ndarray  # uint8 (TILE_N, TILE_N)
    skyline_deg: np.ndarray  # float32 (PROFILE_COUNT,)
    skyline_dist_m: np.ndarray  # float32 (PROFILE_COUNT,)
    sea_share: np.ndarray  # float32 (PROFILE_COUNT,) 0-1
    coast_bearing_deg: float | None


@dataclass
class HorizonResult:
    """Output of :func:`build`."""

    tiles: int
    total_bytes: int
    seconds: float


def curvature_drop(distance_m: np.ndarray | float) -> np.ndarray:
    """Apparent drop (m) of a point at ``distance_m`` (curvature minus refraction)."""
    d = np.asarray(distance_m, dtype=np.float64)
    return d * d * (1.0 - REFRACTION) / (2.0 * EARTH_RADIUS_M)


def skyline_profile(
    height_at,
    eye_m: float,
    count: int = PROFILE_COUNT,
    near_m: float = SKYLINE_NEAR_M,
    far_m: float = SKYLINE_FAR_M,
    step_m: float = 360.0,
    sea_at=None,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Real skyline around a point.

    Args:
        height_at: ``f(east_m, north_m) -> heights`` (vectorised, metres).
        eye_m: Eye altitude (m).
        count: Number of azimuths (0 = north, clockwise).
        near_m: Nearest distance considered (the rendered relief covers the rest).
        far_m: Farthest distance considered.
        step_m: Sampling step along each ray.
        sea_at: Optional ``f(east_m, north_m) -> bool array`` (open sea).

    Returns:
        ``(angle_deg, distance_m, sea_share)`` arrays of length ``count``.
    """
    distances = np.arange(near_m, far_m + 1.0, step_m)
    azimuths = np.arange(count) * (2.0 * math.pi / count)
    east = np.sin(azimuths)[:, None] * distances[None, :]
    north = np.cos(azimuths)[:, None] * distances[None, :]
    heights = np.asarray(height_at(east, north), dtype=np.float64)
    angles = np.degrees(
        np.arctan2(
            heights - eye_m - curvature_drop(distances)[None, :], distances[None, :]
        )
    )
    best = np.argmax(angles, axis=1)
    angle = angles[np.arange(count), best]
    distance = distances[best]
    if sea_at is None:
        share = np.zeros(count)
    else:
        near = distances <= SEA_SHARE_REACH_M
        share = np.asarray(
            sea_at(east[:, near], north[:, near]), dtype=np.float64
        ).mean(axis=1)
    return (
        angle.astype(np.float32),
        distance.astype(np.float32),
        share.astype(np.float32),
    )


def coast_bearing(classes: np.ndarray, step_m: float = STEP_M) -> float | None:
    """Bearing (deg, 0 = north, clockwise) of the sea seen from the tile centre.

    Sea cells are weighted by the inverse of their distance; ``None`` when the
    tile holds less than 1 % sea.
    """
    sea = classes == SEA_CLASS
    if sea.mean() < 0.01:
        return None
    n = classes.shape[0]
    half = (n - 1) / 2.0
    rows, cols = np.nonzero(sea)
    east = (cols - half) * step_m
    north = (half - rows) * step_m
    dist = np.maximum(np.hypot(east, north), step_m)
    ve = float(np.sum(east / dist / dist))
    vn = float(np.sum(north / dist / dist))
    if abs(ve) + abs(vn) < 1e-12:
        return None
    return math.degrees(math.atan2(ve, vn)) % 360.0


def encode_tile(tile: HorizonTile) -> bytes:
    """Binary tile read by ``game/scripts/battle/battle_horizon.gd``.

    Layout (little endian): ``HZR1``, u16 n, u16 profile count, f32 step (m),
    f32 reference altitude (m), u32 size of the inflated payload, then the zlib
    payload: i16[n*n] heights (quarter metres, row 0 = north), u8[n*n] classes,
    i16[p] skyline angle (centidegrees), u16[p] skyline distance (100 m units),
    u8[p] sea share (0-255).
    """
    n = tile.heights_m.shape[0]
    p = tile.skyline_deg.shape[0]
    quarter = np.clip(np.rint(tile.heights_m * 4.0), -32768, 32767).astype("<i2")
    payload = b"".join(
        [
            quarter.tobytes(),
            tile.classes.astype(np.uint8).tobytes(),
            np.clip(np.rint(tile.skyline_deg * 100.0), -32768, 32767)
            .astype("<i2")
            .tobytes(),
            np.clip(np.rint(tile.skyline_dist_m / 100.0), 0, 65535)
            .astype("<u2")
            .tobytes(),
            np.clip(np.rint(tile.sea_share * 255.0), 0, 255).astype(np.uint8).tobytes(),
        ]
    )
    header = MAGIC + struct.pack("<HHffI", n, p, STEP_M, tile.ref_m, len(payload))
    return header + zlib.compress(payload, 9)


def decode_tile(data: bytes) -> dict:
    """Inverse of :func:`encode_tile` (tests, inspection)."""
    assert data[:4] == MAGIC
    n, p, step, ref, size = struct.unpack("<HHffI", data[4:20])
    payload = zlib.decompress(data[20:])
    assert len(payload) == size
    offset = 0
    heights = np.frombuffer(payload, "<i2", n * n, offset).reshape(n, n) / 4.0
    offset += 2 * n * n
    classes = np.frombuffer(payload, np.uint8, n * n, offset).reshape(n, n)
    offset += n * n
    angle = np.frombuffer(payload, "<i2", p, offset) / 100.0
    offset += 2 * p
    dist = np.frombuffer(payload, "<u2", p, offset) * 100.0
    offset += 2 * p
    sea = np.frombuffer(payload, np.uint8, p, offset) / 255.0
    return {
        "n": n,
        "step": step,
        "ref": ref,
        "heights": heights,
        "classes": classes,
        "skyline_deg": angle,
        "skyline_dist_m": dist,
        "sea_share": sea,
    }


# --- Sources -------------------------------------------------------------------------------


class Sources:
    """Rasters shared by every tile (loaded once)."""

    def __init__(self, map_dir: Path = MAP_DIR) -> None:
        """Load the fine relief, land mask, forest channel and province shapes."""
        metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
        self.bounds = tuple(metadata["bounds_projected"])
        self.grid = MapGrid(self.bounds, 4096)
        tiles = metadata["height_tiles"]
        count = tiles["size_px"] // tiles["tile_px"]
        self.fine = terrain.uint16_to_height(
            relief.read_tiles(map_dir / tiles["dir"], count)
        ).astype(np.float32)
        self.fine_mpp = (self.bounds[2] - self.bounds[0]) / self.fine.shape[0]
        self.land = (
            np.asarray(Image.open(map_dir / "land_mask.png"), dtype=np.uint8) > 127
        )
        splat = np.asarray(Image.open(map_dir / "splat.png"), dtype=np.uint8)
        self.forest = splat[:, :, 2].astype(np.float32) / 255.0
        self.forest_mpp = (self.bounds[2] - self.bounds[0]) / self.forest.shape[0]
        self.coast_px = ndimage.distance_transform_edt(self.land)
        geo = json.loads((map_dir / "provinces.geojson").read_text(encoding="utf-8"))
        self.provinces = {f["properties"]["id"]: f for f in geo["features"]}
        self.copernicus_names = copernicus.tiles_in_bbox(
            [p.stem for p in copernicus.RAW_DIR.glob("*.tif")]
        )

    def _sample(
        self,
        array: np.ndarray,
        mpp: float,
        x: np.ndarray,
        y: np.ndarray,
        order: int = 1,
    ) -> np.ndarray:
        minx, _, _, maxy = self.bounds
        col = (np.asarray(x) - minx) / mpp - 0.5
        row = (maxy - np.asarray(y)) / mpp - 0.5
        return ndimage.map_coordinates(array, [row, col], order=order, mode="nearest")

    def fine_height(self, x: np.ndarray, y: np.ndarray) -> np.ndarray:
        """Fine relief (m) at EPSG:3035 coordinates (bilinear)."""
        return self._sample(self.fine, self.fine_mpp, x, y)

    def open_sea(self, x: np.ndarray, y: np.ndarray) -> np.ndarray:
        """Coarse sea mask (4096 land mask, nearest) at EPSG:3035 coordinates."""
        return ~self._sample(
            self.land.astype(np.uint8), self.grid.meters_per_px, x, y, 0
        ).astype(bool)

    def forest_share(self, x: np.ndarray, y: np.ndarray) -> np.ndarray:
        """Forest share (0-1) at EPSG:3035 coordinates (bilinear)."""
        return self._sample(self.forest, self.forest_mpp, x, y)

    def point_of(self, province: str, coastal: bool) -> tuple[float, float]:
        """Representative point (4096 map pixels) of a province."""
        feature = self.provinces[province]
        cx, cy = feature["properties"]["centroid"]
        if not coastal:
            return float(cx), float(cy)
        polygon = shape(feature["geometry"])
        lo, hi = COAST_TARGET_PX
        rows, cols = np.nonzero(
            self.land & (self.coast_px >= lo) & (self.coast_px <= hi)
        )
        order = np.argsort((cols + 0.5 - cx) ** 2 + (rows + 0.5 - cy) ** 2)
        for index in order[:4000]:
            px, py = float(cols[index]) + 0.5, float(rows[index]) + 0.5
            if polygon.contains(Point(px, py)):
                return px, py
        return float(cx), float(cy)


def bake_tile(sources: Sources, province: str, coastal: bool) -> HorizonTile:
    """Bake one province tile."""
    px, py = sources.point_of(province, coastal)
    cx, cy = sources.grid.pixel_to_projected(px, py)
    cx, cy = float(cx), float(cy)
    lon, lat = sources.grid.pixel_to_lonlat(px, py)
    half = TILE_SPAN_M / 2.0
    offsets = np.arange(TILE_N) * STEP_M - half
    xs = cx + offsets[None, :].repeat(TILE_N, 0)
    ys = cy - offsets[:, None].repeat(TILE_N, 1)
    grid = MapGrid(
        (
            cx - half - STEP_M / 2,
            cy - half - STEP_M / 2,
            cx + half + STEP_M / 2,
            cy + half + STEP_M / 2,
        ),
        TILE_N,
    )
    cop = copernicus.resample_to_grid(
        grid, (0, 0, TILE_N, TILE_N), sources.copernicus_names
    )
    fine = sources.fine_height(xs, ys)
    coarse_sea = sources.open_sea(xs, ys)
    has_cop = ~np.isnan(cop)
    cop_sea = has_cop & (np.abs(np.nan_to_num(cop)) <= 0.01)
    # Sea: Copernicus stores the sea surface as 0 m (and has no tile over open water);
    # without Copernicus data, trust the fine relief and the land mask.
    sea = np.where(has_cop, cop_sea & (fine < 5.0), coarse_sea & (fine <= 0.5))
    # Copernicus missing over open sea next to tiles: NaN and low fine relief.
    sea |= ~has_cop & (fine <= 0.0) & coarse_sea
    heights = np.where(has_cop, np.nan_to_num(cop), fine).astype(np.float32)
    heights = np.where(sea, np.minimum(heights, 0.0) - 3.0, heights).astype(np.float32)
    forest = np.clip(sources.forest_share(xs, ys), 0.0, 1.0)
    classes = np.rint(forest * FOREST_MAX).astype(np.uint8)
    classes[sea] = SEA_CLASS
    centre = TILE_N // 2
    fx = int(FIELD_HALF_M[0] / STEP_M)
    fz = int(FIELD_HALF_M[1] / STEP_M)
    footprint = heights[centre - fz : centre + fz + 1, centre - fx : centre + fx + 1]
    land = footprint[footprint > -1.0]
    ref = float(land.mean()) if land.size else 0.0

    def height_at(east: np.ndarray, north: np.ndarray) -> np.ndarray:
        return sources.fine_height(cx + east, cy + north)

    def sea_at(east: np.ndarray, north: np.ndarray) -> np.ndarray:
        x, y = cx + east, cy + north
        return sources.open_sea(x, y) & (sources.fine_height(x, y) <= 0.5)

    angle, dist, share = skyline_profile(
        height_at, max(ref, 0.0) + EYE_M, sea_at=sea_at
    )
    return HorizonTile(
        province=province,
        lon=float(lon),
        lat=float(lat),
        ref_m=ref,
        heights_m=heights,
        classes=classes,
        skyline_deg=angle,
        skyline_dist_m=dist,
        sea_share=share,
        coast_bearing_deg=coast_bearing(classes),
    )


def province_list(provinces_dir: Path = PROVINCES_DIR) -> list[tuple[str, bool, str]]:
    """``(id, coastal, terrain)`` of every province file."""
    result = []
    for path in sorted(provinces_dir.glob("prov_*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        result.append(
            (data["id"], bool(data.get("coastal", False)), str(data.get("terrain", "")))
        )
    return result


def build(out_dir: Path = OUT_DIR, only: list[str] | None = None) -> HorizonResult:
    """Bake every province tile and ``index.json`` (``only``: subset, index merged)."""
    started = time.perf_counter()
    out_dir.mkdir(parents=True, exist_ok=True)
    sources = Sources()
    index_path = out_dir / "index.json"
    index = (
        json.loads(index_path.read_text(encoding="utf-8"))
        if only and index_path.exists()
        else {"provinces": {}}
    )
    index.update(
        {
            "description": "Relief réel autour de chaque province (lot EP2, `cent-ans geo horizon`) : tuiles <province>.bin (voir tools/cent_ans_tools/geo/horizon.py).",
            "tile_span_m": TILE_SPAN_M,
            "step_m": STEP_M,
            "samples": TILE_N,
            "profile_count": PROFILE_COUNT,
            "eye_m": EYE_M,
            "source": "Copernicus DEM GLO-90 (moyenne de zone), tuiles fines 360 m hors emprise ; forêts splat.png",
        }
    )
    total = 0
    count = 0
    for province, coastal, terrain_key in province_list():
        if province not in sources.provinces or (only and province not in only):
            continue
        tile = bake_tile(sources, province, coastal)
        data = encode_tile(tile)
        path = out_dir / f"{province}.bin"
        path.write_bytes(data)
        total += len(data)
        count += 1
        index["provinces"][province] = {
            "file": path.name,
            "lon": round(tile.lon, 4),
            "lat": round(tile.lat, 4),
            "ref_m": round(tile.ref_m, 1),
            "terrain": terrain_key,
            "coastal": coastal,
            "coast_bearing_deg": None
            if tile.coast_bearing_deg is None
            else round(tile.coast_bearing_deg, 1),
            "skyline_max_deg": round(float(tile.skyline_deg.max()), 2),
            "relief_max_m": round(float(tile.heights_m.max()), 0),
            "sea_share": round(float((tile.classes == SEA_CLASS).mean()), 3),
        }
    index["provinces"] = dict(sorted(index["provinces"].items()))
    index_path.write_text(
        json.dumps(index, indent=1, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    return HorizonResult(count, total, time.perf_counter() - started)
