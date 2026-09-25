"""Settlements, hamlets, bridges and roads fitted to the fine relief (lot ZG5a).

``cent-ans geo anchors-fine`` (after ``geo hydro-fine``) writes:

``data/map/fine_anchors.json`` (versioned)
    For each settlement (``settlements_px.json``), hamlet (``hamlets.json``) and
    crossing (``crossings_px.json``): a refined position in world units and an
    altitude on the finest cached relief. A settlement or hamlet only moves (at most
    :data:`MAX_MOVE_M`) when its rule position lies in a fine river bed or on a
    steep slope; a bridge, ford or ferry is put exactly on its fine river (nearest
    line of the same name, else the nearest river), with the river direction, width,
    water level and deck level. Rule positions (``settlements_px.json``...) are
    unchanged: the renderer offsets its models only.
``data/map/pyramid/roads_fine/E2/{col}_{row}.bin`` (cache, CAFV layer 2)
    The roads of ``roads.geojson`` densified, shifted sideways by at most
    :data:`ROAD_RADIUS_M` to ease grades and leave river beds and wet valley
    floors, draped (altitude per vertex, slopes smoothed, cut and fill ≤ 3 m,
    bridges level between their banks), cut along the E2 tiles.

Rendering data only: no game rule reads these files.
"""

from __future__ import annotations

import hashlib
import json
import os
import time
from collections import OrderedDict, defaultdict
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

import numpy as np
from PIL import Image
from pyproj import Transformer
from scipy.spatial import cKDTree

from cent_ans_tools.geo import (
    fine_relief,
    fine_tiles,
    hydro_fine,
    hydro_sources,
    pyramid,
)
from cent_ans_tools.geo import valley_snap as vs

MAP_DIR = hydro_fine.MAP_DIR
ANCHORS_FILE = "fine_anchors.json"
ROADS_SUBDIR = "roads_fine"
ROAD_CACHE = hydro_sources.CACHE_DIR / "roads"
ROAD_VERSION = 2

MAX_MOVE_M = 300.0
SITE_STEP_M = 25.0
SITE_SLOPE_OK = 0.2  # a site flatter than 20 % outside the river bed stays put
BRIDGE_NAMED_M = 2000.0  # search radius for the named river around a crossing
BRIDGE_ANY_M = 500.0
DENSIFY_M = 8.0

ROAD_STEP_CORE_M = 25.0
ROAD_STEP_M = 90.0
ROAD_RADIUS_M = 50.0
ROAD_OFFSETS = 11
ROAD_WIDTH_M = {"main": 6.0, "secondary": 4.0, "computed": 3.0}
ROAD_CHUNK_M = 2_500_000.0


# ------------------------------------------------------------------ geometry


@dataclass(frozen=True)
class Frame:
    """World units (map pixels 4096) <-> EPSG:3035 metres."""

    bounds: tuple[float, float, float, float]

    @property
    def mpp(self) -> float:
        """Metres per world unit."""
        return (self.bounds[2] - self.bounds[0]) / 4096.0

    def to_m(self, units: np.ndarray) -> np.ndarray:
        """World units ``(n, 2)`` to metres."""
        units = np.asarray(units, dtype=np.float64).reshape(-1, 2)
        return np.column_stack(
            [
                self.bounds[0] + units[:, 0] * self.mpp,
                self.bounds[3] - units[:, 1] * self.mpp,
            ]
        )

    def to_units(self, metres: np.ndarray) -> np.ndarray:
        """Metres ``(n, 2)`` to world units."""
        metres = np.asarray(metres, dtype=np.float64).reshape(-1, 2)
        return np.column_stack(
            [
                (metres[:, 0] - self.bounds[0]) / self.mpp,
                (self.bounds[3] - metres[:, 1]) / self.mpp,
            ]
        )


class RiverIndex:
    """Nearest fine river vertex (densified), tile by tile, in metres."""

    def __init__(
        self, map_dir: Path, frame: Frame, names: list[str], cache: int = 24
    ) -> None:
        """Lazy loader of the ``hydro_fine`` tiles."""
        self.dir = map_dir / pyramid.PYRAMID_DIR_NAME / hydro_fine.TILES_SUBDIR / "E2"
        self.frame = frame
        self.names = names
        self.side = fine_tiles.tile_units(fine_tiles.TILE_LEVEL)
        self.cache: OrderedDict[tuple[int, int], dict | None] = OrderedDict()
        self.cache_size = cache
        widths = json.loads(
            (map_dir / hydro_fine.WIDTHS_FILE).read_text(encoding="utf-8")
        )
        self.alias_groups = [
            {hydro_fine.normalise_name(n) for n in river["names"]}
            for river in widths["rivers"]
        ]

    def _tile(self, col: int, row: int) -> dict | None:
        key = (col, row)
        if key in self.cache:
            self.cache.move_to_end(key)
            return self.cache[key]
        path = self.dir / f"{col}_{row}.bin"
        entry = None
        if path.exists():
            decoded = fine_tiles.decode(path.read_bytes())
            pts, width, level, feature, tangent = [], [], [], [], []
            for line in decoded["lines"]:
                metres = self.frame.to_m(line["xy"])
                dense = vs.resample(metres, DENSIFY_M) if len(metres) > 1 else metres
                along = vs.chainage(metres)
                dense_along = np.linspace(0.0, along[-1], len(dense))
                pts.append(dense)
                width.append(np.interp(dense_along, along, line["w"]))
                level.append(np.interp(dense_along, along, line["z"]))
                feature.append(np.full(len(dense), line["feature"]))
                grad = (
                    np.gradient(dense, axis=0)
                    if len(dense) > 1
                    else np.array([[1.0, 0.0]])
                )
                norm = np.maximum(np.hypot(grad[:, 0], grad[:, 1]), 1e-9)
                tangent.append(grad / norm[:, None])
            if pts:
                points = np.concatenate(pts)
                entry = {
                    "tree": cKDTree(points),
                    "points": points,
                    "width": np.concatenate(width),
                    "level": np.concatenate(level),
                    "feature": np.concatenate(feature),
                    "tangent": np.concatenate(tangent),
                }
        self.cache[key] = entry
        while len(self.cache) > self.cache_size:
            self.cache.popitem(last=False)
        return entry

    def aliases(self, name: str) -> set[str]:
        """Normalised names of a river, with the aliases of ``river_widths.json``."""
        key = hydro_fine.normalise_name(name)
        for group in self.alias_groups:
            if key in group:
                return group
        return {key}

    def tiles_around(self, metres: np.ndarray, margin_m: float) -> list[dict]:
        """Loaded tiles overlapping the box of ``metres`` grown by ``margin_m``."""
        lo = self.frame.to_units(metres.min(axis=0) - margin_m)
        hi = self.frame.to_units(metres.max(axis=0) + margin_m)
        c0, c1 = sorted((int(lo[0, 0] // self.side), int(hi[0, 0] // self.side)))
        r0, r1 = sorted((int(lo[0, 1] // self.side), int(hi[0, 1] // self.side)))
        out = []
        for row in range(r0, r1 + 1):
            for col in range(c0, c1 + 1):
                tile = self._tile(col, row)
                if tile is not None:
                    out.append(tile)
        return out

    def bed_distance(
        self, x: np.ndarray, y: np.ndarray, margin_m: float = 200.0
    ) -> np.ndarray:
        """Distance (m) to the nearest river bank (negative inside a bed)."""
        points = np.column_stack([np.ravel(x), np.ravel(y)])
        best = np.full(len(points), np.inf)
        if len(points) == 0:
            return best.reshape(np.shape(x))
        for tile in self.tiles_around(points, margin_m):
            dist, index = tile["tree"].query(points, distance_upper_bound=margin_m)
            ok = np.isfinite(dist)
            bank = np.full(len(points), np.inf)
            bank[ok] = dist[ok] - 0.5 * tile["width"][index[ok]]
            best = np.minimum(best, bank)
        return best.reshape(np.shape(x))

    def nearest(
        self, point: np.ndarray, radius_m: float, name: str | None = None
    ) -> dict | None:
        """Nearest river vertex (optionally of a named river) within ``radius_m``."""
        wanted = self.aliases(name) if name else None
        best = None
        for tile in self.tiles_around(point.reshape(1, 2), radius_m):
            indices = tile["tree"].query_ball_point(point, radius_m)
            if not indices:
                continue
            indices = np.asarray(indices)
            if wanted:
                names = [
                    hydro_fine.normalise_name(self.names[f])
                    for f in tile["feature"][indices]
                ]
                indices = indices[
                    [n in wanted or any(w in n.split() for w in wanted) for n in names]
                ]
                if len(indices) == 0:
                    continue
            dist = np.hypot(*(tile["points"][indices] - point).T)
            k = int(np.argmin(dist))
            if best is None or dist[k] < best["distance"]:
                i = int(indices[k])
                best = {
                    "distance": float(dist[k]),
                    "point": tile["points"][i],
                    "width": float(tile["width"][i]),
                    "level": float(tile["level"][i]),
                    "feature": int(tile["feature"][i]),
                    "tangent": tile["tangent"][i],
                }
        return best


# ---------------------------------------------------------------- site search


def site_candidates(centre: np.ndarray, radius: float, step: float) -> np.ndarray:
    """Grid points of a disc (the centre first)."""
    count = int(radius // step)
    offsets = np.arange(-count, count + 1) * step
    gx, gy = np.meshgrid(offsets, offsets)
    inside = gx**2 + gy**2 <= radius**2
    cand = np.column_stack([gx[inside], gy[inside]])
    order = np.argsort(np.hypot(cand[:, 0], cand[:, 1]), kind="stable")
    return centre + cand[order]


def slope_at(
    relief: fine_relief.FineRelief, points: np.ndarray, delta: float = 15.0
) -> np.ndarray:
    """Slope (rise over run) from central differences over ``delta`` metres."""
    x, y = points[:, 0], points[:, 1]
    hx = relief.sample(x + delta, y) - relief.sample(x - delta, y)
    hy = relief.sample(x, y + delta) - relief.sample(x, y - delta)
    return np.hypot(hx, hy) / (2.0 * delta)


def fit_site(
    relief: fine_relief.FineRelief, rivers: RiverIndex, centre: np.ndarray
) -> tuple[np.ndarray, float, float, str]:
    """Refined site: ``(point, altitude, moved metres, reason)``."""
    here = centre.reshape(1, 2)
    slope0 = float(slope_at(relief, here)[0])
    bank0 = float(rivers.bed_distance(here[:, 0], here[:, 1])[0])
    if bank0 > 0.0 and (slope0 <= SITE_SLOPE_OK or not np.isfinite(slope0)):
        return centre, float(relief.sample(here[:, 0], here[:, 1])[0]), 0.0, ""
    cand = site_candidates(centre, MAX_MOVE_M, SITE_STEP_M)
    slope = slope_at(relief, cand)
    bank = rivers.bed_distance(cand[:, 0], cand[:, 1])
    dist = np.hypot(*(cand - centre).T)
    cost = np.nan_to_num(slope, nan=1.0) / 0.04 + dist / 150.0
    cost += np.where(bank < 0.0, 50.0, 0.0) + np.where(bank < 15.0, 3.0, 0.0)
    k = int(np.argmin(cost))
    reason = "lit" if bank0 <= 0.0 else "pente"
    point = cand[k]
    altitude = float(relief.sample(point[:1], point[1:])[0])
    return point, altitude, float(dist[k]), reason if dist[k] > 0 else ""


def fit_crossing(
    relief: fine_relief.FineRelief,
    rivers: RiverIndex,
    position: np.ndarray,
    river_name: str,
) -> dict:
    """A bridge, ford or ferry put exactly on its fine river."""
    hit = rivers.nearest(position, BRIDGE_NAMED_M, river_name) if river_name else None
    if hit is None:
        hit = rivers.nearest(position, BRIDGE_ANY_M)
    if hit is None:
        altitude = float(relief.sample(position[:1], position[1:])[0])
        return {"point": position, "snapped": False, "z_ground": altitude}
    tangent = hit["tangent"]
    normal = np.array([-tangent[1], tangent[0]])
    reach = 0.5 * hit["width"] + 8.0
    banks = np.vstack([hit["point"] + normal * reach, hit["point"] - normal * reach])
    bank_z = relief.sample(banks[:, 0], banks[:, 1])
    return {
        "point": hit["point"],
        "snapped": True,
        "feature": hit["feature"],
        "tangent": tangent,
        "width_m": hit["width"],
        "z_water": hit["level"],
        "z_banks": bank_z,
        "moved_m": float(np.hypot(*(hit["point"] - position))),
    }


# ----------------------------------------------------------------------- roads


_ROAD_WORKER: dict = {}


def _init_road_worker(map_dir: str, bounds: tuple[float, float, float, float]) -> None:
    frame = Frame(bounds)
    features = json.loads(
        (
            Path(map_dir)
            / pyramid.PYRAMID_DIR_NAME
            / hydro_fine.TILES_SUBDIR
            / hydro_fine.FEATURES_FILE
        ).read_text(encoding="utf-8")
    )["features"]
    _ROAD_WORKER["frame"] = frame
    _ROAD_WORKER["relief"] = fine_relief.FineRelief(
        Path(map_dir), bounds, cache_tiles=128
    )
    _ROAD_WORKER["rivers"] = RiverIndex(
        Path(map_dir), frame, [f["name"] for f in features]
    )
    wet = np.asarray(
        Image.open(Path(map_dir) / "wetlands.png").convert("RGB"), dtype=np.float32
    )
    _ROAD_WORKER["wet"] = np.clip(wet.max(axis=2) / 255.0 * 4.0, 0.0, 1.0)


def wet_at(wet: np.ndarray, frame: Frame, x: np.ndarray, y: np.ndarray) -> np.ndarray:
    """Wetness 0-1 of ``wetlands.png`` (719 m pixels, nearest)."""
    u = np.clip(
        ((np.asarray(x) - frame.bounds[0]) / frame.mpp).astype(np.int64), 0, 4095
    )
    v = np.clip(
        ((frame.bounds[3] - np.asarray(y)) / frame.mpp).astype(np.int64), 0, 4095
    )
    return wet[v, u]


def drape_road(
    relief: fine_relief.FineRelief,
    rivers: RiverIndex,
    wet: np.ndarray,
    frame: Frame,
    metres: np.ndarray,
    core: bool,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Route and drape one road; returns points, altitudes and a per-vertex causeway flag."""
    step = ROAD_STEP_CORE_M if core else ROAD_STEP_M
    points = vs.resample(metres, step)

    def node_cost(x: np.ndarray, y: np.ndarray, heights: np.ndarray) -> np.ndarray:
        bank = rivers.bed_distance(x, y)
        lowness = np.clip(
            1.0 - (heights - heights.min(axis=1, keepdims=True)) / 3.0, 0.0, 1.0
        )
        return (
            6.0 * (bank < 0.0)
            + 1.5 * (bank < 10.0)
            + 3.0 * wet_at(wet, frame, x, y) * lowness
        )

    if core:
        routed, heights, _ = vs.lateral_route(
            points,
            relief.sample,
            node_cost,
            ROAD_RADIUS_M,
            offsets=ROAD_OFFSETS,
            prior_m=1.0,
            lateral_cost=0.04,
            grade_cost=0.6,
        )
    else:
        routed = points
        heights = relief.sample(points[:, 0], points[:, 1])
    heights = np.where(
        np.isfinite(heights),
        heights,
        np.nanmean(heights) if np.isfinite(heights).any() else 0.0,
    )
    bank = rivers.bed_distance(routed[:, 0], routed[:, 1])
    in_bed = bank < 0.0
    if in_bed.any() and (~in_bed).any():
        # Bridges: level deck between the banks.
        index = np.arange(len(heights))
        heights = heights.copy()
        heights[in_bed] = np.interp(index[in_bed], index[~in_bed], heights[~in_bed])
    draped = vs.drape(heights, step, smooth_m=60.0 if core else 150.0, max_cut_m=3.0)
    wet_low = wet_at(wet, frame, routed[:, 0], routed[:, 1]) > 0.25
    return routed, draped, wet_low & ~in_bed


def _road_chunk(job: dict) -> str:
    frame = _ROAD_WORKER["frame"]
    counts, xy, z, causeway = [], [], [], []
    for metres, core in zip(job["lines"], job["core"], strict=True):
        routed, heights, flags = drape_road(
            _ROAD_WORKER["relief"],
            _ROAD_WORKER["rivers"],
            _ROAD_WORKER["wet"],
            frame,
            metres,
            core,
        )
        keep = vs.simplify_indices_3d(routed, heights, 2.5 if core else 8.0, 4.0)
        counts.append(len(keep))
        xy.append(routed[keep])
        z.append(heights[keep])
        causeway.append(flags[keep])
    out = Path(job["out"])
    tmp = out.with_name(out.stem + ".tmp.npz")
    np.savez(
        tmp,
        counts=np.asarray(counts, dtype=np.int64),
        xy=np.concatenate(xy) if xy else np.zeros((0, 2)),
        z=np.concatenate(z) if z else np.zeros(0),
        causeway=np.concatenate(causeway) if causeway else np.zeros(0, bool),
    )
    tmp.replace(out)
    return str(out)


def build_roads(
    map_dir: Path,
    frame: Frame,
    relief: fine_relief.FineRelief,
    workers: int,
    log=print,  # noqa: ANN001
) -> tuple[list[dict], dict]:
    """Drape every road of ``roads.geojson`` (parallel, resumable) and write the tiles."""
    features = json.loads((map_dir / "roads.geojson").read_text(encoding="utf-8"))[
        "features"
    ]
    lines, meta = [], []
    for index, feature in enumerate(features):
        for part in hydro_sources_lines(feature.get("geometry") or {}):
            if len(part) < 2:
                continue
            metres = frame.to_m(np.asarray(part, dtype=np.float64))
            lines.append(metres)
            props = feature.get("properties") or {}
            meta.append((index, str(props.get("type") or "secondary")))
    mids = np.array([line[len(line) // 2] for line in lines])
    core = hydro_fine.core_mask(relief, mids)
    order = sorted(
        range(len(lines)), key=lambda i: (round(mids[i][1] / 46000.0), mids[i][0])
    )
    chunks, current, total = [], [], 0.0
    for i in order:
        current.append(i)
        total += vs.polyline_length(lines[i]) * (4.0 if core[i] else 1.0)
        if total > ROAD_CHUNK_M:
            chunks.append(current)
            current, total = [], 0.0
    if current:
        chunks.append(current)
    ROAD_CACHE.mkdir(parents=True, exist_ok=True)
    river_stamp = (
        (map_dir / hydro_fine.MANIFEST_FILE).stat().st_mtime_ns
        if (map_dir / hydro_fine.MANIFEST_FILE).exists()
        else 0
    )
    jobs = []
    for k, chunk in enumerate(chunks):
        digest = hashlib.sha1(f"{ROAD_VERSION}|{river_stamp}".encode())
        for i in chunk:
            digest.update(np.ascontiguousarray(lines[i][[0, -1]]).tobytes())
        jobs.append(
            {
                "out": str(ROAD_CACHE / f"roads_{k:04d}_{digest.hexdigest()[:10]}.npz"),
                "chunk": chunk,
                "lines": [lines[i] for i in chunk],
                "core": [bool(core[i]) for i in chunk],
            }
        )
    todo = [job for job in jobs if not Path(job["out"]).exists()]
    log(f"routes : {len(lines)} tronçons, {len(jobs)} lots, {len(todo)} à draper")
    if todo:
        started = time.time()
        with ProcessPoolExecutor(
            max_workers=workers,
            initializer=_init_road_worker,
            initargs=(str(map_dir), frame.bounds),
        ) as pool:
            futures = [pool.submit(_road_chunk, job) for job in todo]
            for done, future in enumerate(as_completed(futures), 1):
                future.result()
                if done % max(1, len(todo) // 10) == 0 or done == len(todo):
                    log(
                        f"  routes : {done}/{len(todo)} lots ({time.time() - started:.0f} s)"
                    )
    side = fine_tiles.tile_units(fine_tiles.TILE_LEVEL)
    limit = int(round(4096 / side))
    tiles: dict[tuple[int, int], fine_tiles.TileLines] = defaultdict(
        fine_tiles.TileLines
    )
    total_km = 0.0
    for job in jobs:
        data = np.load(job["out"])
        splits = np.cumsum(data["counts"])[:-1]
        for i, xy, z, causeway in zip(
            job["chunk"],
            np.split(data["xy"], splits),
            np.split(data["z"], splits),
            np.split(data["causeway"], splits),
            strict=True,
        ):
            if len(xy) < 2:
                continue
            total_km += vs.polyline_length(xy) / 1000.0
            feature, kind = meta[i]
            width = ROAD_WIDTH_M.get(kind, 4.0)
            units = frame.to_units(xy)
            attrs = np.column_stack([z, causeway.astype(np.float64)])
            for col, row, pts, att in fine_tiles.split_by_tiles(units, attrs, side):
                if not (0 <= col < limit and 0 <= row < limit):
                    continue
                flags = 0
                if kind == "main":
                    flags |= fine_tiles.FLAG_MAIN_ROAD
                if kind == "computed":
                    flags |= fine_tiles.FLAG_COMPUTED_ROAD
                if np.mean(att[:, 1] > 0.5) >= 0.5:
                    flags |= fine_tiles.FLAG_WETLAND
                tiles[(col, row)].add(
                    feature, flags, pts, att[:, 0], np.full(len(pts), width)
                )
    directory = (
        map_dir / pyramid.PYRAMID_DIR_NAME / ROADS_SUBDIR / f"E{fine_tiles.TILE_LEVEL}"
    )
    index = fine_tiles.write_tiles(
        directory, fine_tiles.LAYER_ROADS, fine_tiles.TILE_LEVEL, dict(tiles)
    )
    stats = {
        "segments": len(lines),
        "km": round(total_km, 1),
        "tiles": len(index),
        "points": sum(e["points"] for e in index),
        "total_bytes": sum(e["bytes"] for e in index),
    }
    return index, stats


def hydro_sources_lines(geometry: dict) -> list[list[list[float]]]:
    """LineString / MultiLineString coordinates as a list of lines."""
    if geometry.get("type") == "LineString":
        return [geometry["coordinates"]]
    if geometry.get("type") == "MultiLineString":
        return list(geometry["coordinates"])
    return []


# ----------------------------------------------------------------------- build


@dataclass
class AnchorsResult:
    """Summary of ``geo anchors-fine``."""

    path: Path
    settlements: int
    settlements_moved: int
    hamlets: int
    hamlets_moved: int
    crossings: int
    crossings_snapped: int
    crossing_move_median_m: float
    road_stats: dict
    seconds: float

    def summary(self) -> str:
        """One paragraph for the command line."""
        return (
            f"{self.settlements} colonies ({self.settlements_moved} déplacées), "
            f"{self.hamlets} hameaux ({self.hamlets_moved} déplacés), "
            f"{self.crossings} passages ({self.crossings_snapped} sur un fleuve fin, "
            f"déplacement médian {self.crossing_move_median_m:.0f} m) ; routes : "
            f"{self.road_stats.get('km', 0):.0f} km, {self.road_stats.get('points', 0)} points, "
            f"{self.road_stats.get('total_bytes', 0) / 1e6:.1f} Mo ; {self.seconds:.0f} s → {self.path}"
        )


def _round_units(point_m: np.ndarray, frame: Frame) -> list[float]:
    units = frame.to_units(point_m)[0]
    return [round(float(units[0]), 4), round(float(units[1]), 4)]


def build(
    map_dir: Path = MAP_DIR,
    workers: int | None = None,
    log=print,  # noqa: ANN001
) -> AnchorsResult:
    """Write ``fine_anchors.json`` and the draped road tiles."""
    started = time.time()
    workers = workers or max(1, (os.cpu_count() or 2) - 1)
    frame = Frame(pyramid.map_bounds(map_dir))
    relief = fine_relief.FineRelief(map_dir, frame.bounds)
    features_path = (
        map_dir
        / pyramid.PYRAMID_DIR_NAME
        / hydro_fine.TILES_SUBDIR
        / hydro_fine.FEATURES_FILE
    )
    if not features_path.exists():
        raise FileNotFoundError("lancer d'abord `cent-ans geo hydro-fine`")
    names = [
        f["name"]
        for f in json.loads(features_path.read_text(encoding="utf-8"))["features"]
    ]
    rivers = RiverIndex(map_dir, frame, names, cache=64)

    settlements = json.loads(
        (map_dir / "settlements_px.json").read_text(encoding="utf-8")
    )
    out_settlements = {}
    moved_settlements = 0
    for sid in sorted(settlements):
        centre = frame.to_m(np.asarray(settlements[sid]))[0]
        point, altitude, moved, reason = fit_site(relief, rivers, centre)
        moved_settlements += moved > 0
        entry = {"px": _round_units(point, frame), "z": round(altitude, 2)}
        if moved > 0:
            entry["moved_m"] = round(moved, 1)
            entry["reason"] = reason
        out_settlements[sid] = entry
    log(f"colonies : {len(out_settlements)} ({moved_settlements} déplacées)")

    hamlets = json.loads((map_dir / "hamlets.json").read_text(encoding="utf-8"))
    out_hamlets = []
    moved_hamlets = 0
    for hamlet in hamlets:
        centre = frame.to_m(np.asarray(hamlet["px"]))[0]
        point, altitude, moved, _ = fit_site(relief, rivers, centre)
        moved_hamlets += moved > 0
        units = _round_units(point, frame)
        out_hamlets.append([units[0], units[1], round(altitude, 2), round(moved, 1)])
    log(f"hameaux : {len(out_hamlets)} ({moved_hamlets} déplacés)")

    transformer = Transformer.from_crs("EPSG:4326", "EPSG:3035", always_xy=True)
    historical = {
        c["id"]: c["lonlat"]
        for c in json.loads((map_dir / "crossings.json").read_text(encoding="utf-8"))[
            "crossings"
        ]
        if "lonlat" in c
    }
    crossings = json.loads((map_dir / "crossings_px.json").read_text(encoding="utf-8"))[
        "crossings"
    ]
    out_crossings = []
    moves = []
    for crossing in crossings:
        if crossing["id"] in historical:
            position = np.array(transformer.transform(*historical[crossing["id"]]))
        else:
            position = frame.to_m(np.asarray(crossing["px"]))[0]
        fit = fit_crossing(relief, rivers, position, crossing.get("river", ""))
        entry = {
            "id": crossing["id"],
            "px": _round_units(fit["point"], frame),
            "snapped": fit["snapped"],
        }
        if fit["snapped"]:
            tangent_units = np.array([fit["tangent"][0], -fit["tangent"][1]])
            entry.update(
                {
                    "river_feature": fit["feature"],
                    "dir": [round(float(v), 4) for v in tangent_units],
                    "width_m": round(fit["width_m"], 1),
                    "z_water": round(fit["z_water"], 2),
                    "z_deck": round(float(np.nanmax(fit["z_banks"])), 2),
                    "moved_m": round(fit["moved_m"], 1),
                }
            )
            moves.append(fit["moved_m"])
        else:
            entry["z_ground"] = round(fit["z_ground"], 2)
        out_crossings.append(entry)
    log(f"passages : {len(out_crossings)} ({len(moves)} sur un fleuve fin)")

    road_index, road_stats = build_roads(map_dir, frame, relief, workers, log)
    payload = {
        "description": "Ancrages sur le relief fin (lot ZG5a, ADR 0036). Généré par `cent-ans geo anchors-fine` : ne pas modifier à la main. Positions en unités monde (pixels carte 4096), altitudes en mètres sur l'étage le plus fin de la pyramide (non exagérées). Les positions de règles (settlements_px.json, hamlets.json, crossings_px.json) ne changent pas : le rendu seul décale ses modèles. Rendu seulement, aucune règle de jeu.",
        "version": 1,
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat(),
        "max_move_m": MAX_MOVE_M,
        "settlements": out_settlements,
        "hamlets": {
            "fields": ["x", "y", "z", "moved_m"],
            "order": "hamlets.json",
            "items": out_hamlets,
        },
        "crossings": out_crossings,
        "roads": {
            "format": "CAFV",
            "layer": fine_tiles.LAYER_ROADS,
            "tile_level": fine_tiles.TILE_LEVEL,
            "dir": f"{pyramid.PYRAMID_DIR_NAME}/{ROADS_SUBDIR}",
            "pattern": f"E{fine_tiles.TILE_LEVEL}/{{col}}_{{row}}.bin",
            "feature": "index de l'entité dans roads.geojson",
            "width_m": ROAD_WIDTH_M,
            "stats": road_stats,
            "tiles": road_index,
        },
    }
    path = map_dir / ANCHORS_FILE
    path.write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    return AnchorsResult(
        path=path,
        settlements=len(out_settlements),
        settlements_moved=int(moved_settlements),
        hamlets=len(out_hamlets),
        hamlets_moved=int(moved_hamlets),
        crossings=len(out_crossings),
        crossings_snapped=len(moves),
        crossing_move_median_m=float(np.median(moves)) if moves else 0.0,
        road_stats=road_stats,
        seconds=time.time() - started,
    )
