"""Fine river network snapped on the relief pyramid (lot ZG5a, ADR 0036).

``cent-ans geo hydro-fine`` builds, from the national networks of
:mod:`cent_ans_tools.geo.hydro_sources`:

1. link tables per source, with Strahler orders (cached in
   ``tools/geo/raw/hydro/cache/links_<source>.npz``);
2. strokes (river from source to confluence), post-1340 canals removed
   (``data/map/historical_hydro_notes.json``), small streams dropped, each source
   kept where it is authoritative (BD TOPAGE in France, OS Open Rivers in Great
   Britain, EU-Hydro elsewhere in the core, Natural Earth off the core);
3. each stroke snapped onto the valley floor of the finest cached pyramid level
   (:mod:`cent_ans_tools.geo.valley_snap`), in parallel, one cached result per
   chunk (``cache/snap/``) so that an interrupted run resumes;
4. water level made monotonic downstream across the whole network (a tributary
   never ends below the river it joins, whose level is fitted first), confluences
   joined, widths in metres (``data/map/river_widths.json``);
5. tiles ``data/map/pyramid/hydro_fine/E2/{col}_{row}.bin`` (format CAFV, see
   :mod:`cent_ans_tools.geo.fine_tiles`), the feature table
   ``data/map/pyramid/hydro_fine/features.json`` and the versioned manifest
   ``data/map/rivers_fine.json``.

``rivers_render.json``, ``river_bed.png`` and ``crossings*.json`` are not touched.
"""

from __future__ import annotations

import hashlib
import json
import os
import time
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import dataclass, field
from datetime import UTC, datetime
from pathlib import Path

import numpy as np
import shapely
from pyproj import Transformer

from cent_ans_tools.geo import download, fine_relief, fine_tiles, hydro_sources, pyramid
from cent_ans_tools.geo import valley_snap as vs
from cent_ans_tools.geo.project import MapGrid

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
NOTES_FILE = "historical_hydro_notes.json"
WIDTHS_FILE = "river_widths.json"
MANIFEST_FILE = "rivers_fine.json"
TILES_SUBDIR = "hydro_fine"
FEATURES_FILE = "features.json"
SNAP_DIR = hydro_sources.CACHE_DIR / "snap"
#: Bumped whenever the snapping parameters change (invalidates ``cache/snap``).
SNAP_VERSION = 3

#: Smallest Strahler order kept, per source (orders computed on each network).
MIN_ORDER = {"topage": 3, "osor": 3, "euhydro": 3, "naturalearth": 0}
MIN_STROKE_M = 800.0
#: Bounding boxes (lon/lat) queried on EU-Hydro: the core outside France and GB.
EUHYDRO_BOXES = (
    (2.0, 49.0, 8.0, 54.0),  # Low Countries, Luxembourg, Rhineland, Saarland
    (5.8, 45.8, 8.0, 49.0),  # Switzerland (west), Baden, Palatinate
    (6.5, 43.6, 8.0, 45.8),  # Piedmont, Liguria, Monaco
    (-2.0, 42.3, 3.4, 43.4),  # south slope of the Pyrenees, Andorra
)
EUHYDRO_ORDERS = (3, 4, 5, 6, 7, 8, 9)
#: Cells (metres) of the national coverage raster (EU-Hydro dropped there).
COVERAGE_CELL_M = 2000.0
CHUNK_M = 1_500_000.0  # stroke length per parallel work unit
JOIN_BLEND_VERTICES = 6


@dataclass(frozen=True)
class SourceParams:
    """Snapping parameters of one source (radius grows with the order)."""

    step_m: float
    radius_m: dict[int, float]
    offsets: int
    prior_m: float
    lateral_cost: float

    def radius(self, order: int) -> float:
        """Search radius for a stroke of Strahler ``order``."""
        keys = sorted(self.radius_m)
        best = keys[0]
        for key in keys:
            if order >= key:
                best = key
        return self.radius_m[best]


SOURCE_PARAMS = {
    "topage": SourceParams(20.0, {0: 40.0, 4: 60.0, 6: 100.0}, 33, 2.5, 0.08),
    "osor": SourceParams(20.0, {0: 40.0, 4: 60.0, 6: 100.0}, 33, 2.5, 0.08),
    "euhydro": SourceParams(20.0, {0: 60.0, 4: 90.0, 6: 150.0}, 37, 2.0, 0.08),
    "naturalearth": SourceParams(60.0, {0: 1200.0}, 61, 4.0, 0.03),
}
SOURCE_CODES = {"topage": 1, "osor": 2, "euhydro": 3, "naturalearth": 4}


@dataclass
class RiverLine:
    """A snapped stroke ready for tiling."""

    source: str
    name: str
    order: int
    points: np.ndarray  # EPSG:3035
    level: np.ndarray  # water level (m)
    width: np.ndarray  # metres
    intermittent: np.ndarray
    tidal: np.ndarray
    receiver: int = -1  # index of the line it flows into
    orders: np.ndarray = field(default_factory=lambda: np.zeros(0, np.int16))


# ----------------------------------------------------------------- link tables


def links_cache(source: str) -> Path:
    """Cache file of the link table of ``source``."""
    return hydro_sources.CACHE_DIR / f"links_{source}.npz"


def prepare_topage(force: bool = False) -> Path:
    """BD TOPAGE links with Strahler orders (slow: ~3 M links, a few minutes)."""
    path = links_cache("topage")
    if path.exists() and not force:
        return path
    table = hydro_sources.read_topage(hydro_sources.ensure_topage())
    table.strahler = hydro_sources.compute_strahler(table)
    table.save(path)
    return path


def prepare_osor(force: bool = False) -> Path:
    """OS Open Rivers links with Strahler orders."""
    path = links_cache("osor")
    if path.exists() and not force:
        return path
    table = hydro_sources.read_osor(hydro_sources.ensure_osor())
    table.strahler = hydro_sources.compute_strahler(table)
    table.save(path)
    return path


def prepare_euhydro(force: bool = False) -> Path:
    """EU-Hydro links of orders >= 3 over :data:`EUHYDRO_BOXES` (REST, resumable)."""
    path = links_cache("euhydro")
    if path.exists() and not force:
        return path
    cells = hydro_sources.HYDRO_RAW / "euhydro"
    for box in EUHYDRO_BOXES:
        hydro_sources.fetch_euhydro(box, EUHYDRO_ORDERS, cells)
    table = hydro_sources.read_euhydro(cells)
    table.save(path)
    return path


def prepare_naturalearth(map_dir: Path, force: bool = False) -> Path:
    """Natural Earth lines of ``rivers.geojson`` as a link table."""
    path = links_cache("naturalearth")
    if path.exists() and not force:
        return path
    grid = MapGrid(pyramid.map_bounds(map_dir), 4096)
    table = hydro_sources.read_natural_earth(map_dir / "rivers.geojson", grid)
    table.save(path)
    return path


# ------------------------------------------------------------------ selection


def midpoints(table: hydro_sources.LinkTable) -> np.ndarray:
    """Middle vertex of each link."""
    return np.array([line[len(line) // 2] for line in table.lines]).reshape(-1, 2)


def core_mask(relief: fine_relief.FineRelief, points: np.ndarray) -> np.ndarray:
    """Points over the tier-2 core (an E3 tile exists)."""
    if len(points) == 0:
        return np.zeros(0, dtype=bool)
    grid = relief.present.get(3)
    if grid is None or not grid.any():
        return np.zeros(len(points), dtype=bool)
    u, v = relief.level_coords(points[:, 0], points[:, 1], 3)
    col = np.floor((u + 0.5) / fine_relief.TILE_PX).astype(np.int64)
    row = np.floor((v + 0.5) / fine_relief.TILE_PX).astype(np.int64)
    side = grid.shape[0]
    inside = (col >= 0) & (col < side) & (row >= 0) & (row < side)
    out = np.zeros(len(points), dtype=bool)
    out[inside] = grid[row[inside], col[inside]]
    return out


def coverage_raster(
    tables: list[hydro_sources.LinkTable], bounds: tuple[float, float, float, float]
) -> np.ndarray:
    """Cells (``COVERAGE_CELL_M``) reached by the national networks, dilated by one."""
    from scipy import ndimage

    size = int(np.ceil((bounds[2] - bounds[0]) / COVERAGE_CELL_M))
    grid = np.zeros((size, size), dtype=bool)
    for table in tables:
        for line in table.lines:
            col = ((line[:, 0] - bounds[0]) // COVERAGE_CELL_M).astype(np.int64)
            row = ((bounds[3] - line[:, 1]) // COVERAGE_CELL_M).astype(np.int64)
            ok = (col >= 0) & (col < size) & (row >= 0) & (row < size)
            grid[row[ok], col[ok]] = True
    return ndimage.binary_dilation(grid, iterations=1)


def covered(
    raster: np.ndarray, bounds: tuple[float, float, float, float], points: np.ndarray
) -> np.ndarray:
    """Points in covered cells of :func:`coverage_raster`."""
    size = raster.shape[0]
    col = ((points[:, 0] - bounds[0]) // COVERAGE_CELL_M).astype(np.int64)
    row = ((bounds[3] - points[:, 1]) // COVERAGE_CELL_M).astype(np.int64)
    ok = (col >= 0) & (col < size) & (row >= 0) & (row < size)
    out = np.zeros(len(points), dtype=bool)
    out[ok] = raster[row[ok], col[ok]]
    return out


def select_links(
    table: hydro_sources.LinkTable,
    canal_filter: hydro_sources.CanalFilter,
    keep: np.ndarray,
) -> hydro_sources.LinkTable:
    """Links of order >= the source minimum, not modern canals, inside ``keep``."""
    mask = keep & (table.strahler >= MIN_ORDER[table.source])
    mask &= ~canal_filter.excluded(table)
    return table.subset(mask)


# ------------------------------------------------------------------- snapping


def _stroke_order(table: hydro_sources.LinkTable, stroke: hydro_sources.Stroke) -> int:
    return int(max(table.strahler[i] for i in stroke.links))


def _chunk_strokes(
    strokes: list[hydro_sources.Stroke], bounds: tuple[float, float, float, float]
) -> list[list[int]]:
    """Deterministic chunks of strokes, grouped by E2 tile of their first vertex."""
    side = (bounds[2] - bounds[0]) / (16 << 2)

    def key(i: int) -> tuple[int, int, float, float]:
        first = strokes[i].points[0]
        return (
            int((bounds[3] - first[1]) // side),
            int((first[0] - bounds[0]) // side),
            float(first[0]),
            float(first[1]),
        )

    ordered = sorted(range(len(strokes)), key=key)
    chunks: list[list[int]] = [[]]
    total = 0.0
    for i in ordered:
        length = vs.polyline_length(strokes[i].points)
        if total > CHUNK_M and chunks[-1]:
            chunks.append([])
            total = 0.0
        chunks[-1].append(i)
        total += length
    return [c for c in chunks if c]


_WORKER: dict = {}


def _init_worker(map_dir: str, bounds: tuple[float, float, float, float]) -> None:
    _WORKER["relief"] = fine_relief.FineRelief(Path(map_dir), bounds, cache_tiles=160)


def snap_stroke(
    relief: fine_relief.FineRelief,
    points: np.ndarray,
    params: SourceParams,
    order: int,
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """Resample and snap one stroke; returns points, floor heights, source chainage."""
    resampled = vs.resample(points, params.step_m)
    snap = vs.SnapParams(
        radius_m=params.radius(order),
        offsets=params.offsets,
        prior_m=params.prior_m,
        lateral_cost=params.lateral_cost,
    )
    snapped, floor, _ = vs.snap_to_valley(resampled, relief.sample, snap)
    # Chainage of each resampled vertex along the source line (attribute lookup).
    along = np.linspace(0.0, vs.polyline_length(points), len(resampled))
    return snapped, floor, along


def _snap_chunk(job: dict) -> str:
    """Worker: snap the strokes of one chunk and write ``job['out']``."""
    relief = _WORKER["relief"]
    params = SourceParams(**job["params"])
    counts, xy, floor, along = [], [], [], []
    for points, order in zip(job["points"], job["orders"], strict=True):
        snapped, heights, chain = snap_stroke(relief, points, params, order)
        counts.append(len(snapped))
        xy.append(snapped)
        floor.append(heights)
        along.append(chain)
    out = Path(job["out"])
    tmp = out.with_name(out.stem + ".tmp.npz")
    np.savez(
        tmp,
        counts=np.asarray(counts, dtype=np.int64),
        xy=np.concatenate(xy) if xy else np.zeros((0, 2)),
        floor=np.concatenate(floor) if floor else np.zeros(0),
        along=np.concatenate(along) if along else np.zeros(0),
    )
    tmp.replace(out)
    return str(out)


def snap_all(
    source: str,
    strokes: list[hydro_sources.Stroke],
    orders: list[int],
    map_dir: Path,
    bounds: tuple[float, float, float, float],
    workers: int,
    log=print,  # noqa: ANN001
) -> list[tuple[np.ndarray, np.ndarray, np.ndarray]]:
    """Snap every stroke (parallel, resumable); results in stroke order."""
    SNAP_DIR.mkdir(parents=True, exist_ok=True)
    params = SOURCE_PARAMS[source]
    chunks = _chunk_strokes(strokes, bounds)
    jobs = []
    for index, chunk in enumerate(chunks):
        digest = hashlib.sha1()
        digest.update(f"{SNAP_VERSION}|{params}".encode())
        for i in chunk:
            digest.update(np.ascontiguousarray(strokes[i].points[[0, -1]]).tobytes())
            digest.update(str(len(strokes[i].points)).encode())
        out = SNAP_DIR / f"{source}_{index:04d}_{digest.hexdigest()[:10]}.npz"
        jobs.append(
            {
                "index": index,
                "chunk": chunk,
                "out": str(out),
                "params": params.__dict__,
                "points": [strokes[i].points for i in chunk],
                "orders": [orders[i] for i in chunk],
            }
        )
    todo = [job for job in jobs if not Path(job["out"]).exists()]
    log(f"{source} : {len(strokes)} traits, {len(jobs)} lots, {len(todo)} à recaler")
    if todo:
        started = time.time()
        with ProcessPoolExecutor(
            max_workers=workers,
            initializer=_init_worker,
            initargs=(str(map_dir), bounds),
        ) as pool:
            futures = [pool.submit(_snap_chunk, job) for job in todo]
            for done, future in enumerate(as_completed(futures), 1):
                future.result()
                if done % max(1, len(todo) // 10) == 0 or done == len(todo):
                    log(
                        f"  {source} : {done}/{len(todo)} lots ({time.time() - started:.0f} s)"
                    )
    results: list[tuple[np.ndarray, np.ndarray, np.ndarray] | None] = [None] * len(
        strokes
    )
    for job in jobs:
        data = np.load(job["out"])
        splits = np.cumsum(data["counts"])[:-1]
        for i, xy, floor, along in zip(
            job["chunk"],
            np.split(data["xy"], splits),
            np.split(data["floor"], splits),
            np.split(data["along"], splits),
            strict=True,
        ):
            results[i] = (xy, floor, along)
    return results  # type: ignore[return-value]


# -------------------------------------------------------------------- widths


@dataclass
class WidthModel:
    """Widths of ``river_widths.json``."""

    by_order: dict[int, float]
    exponent: float
    search_m: float
    rivers: list[dict]
    names: dict[str, int]

    @classmethod
    def load(cls, path: Path, to_3035) -> WidthModel:  # noqa: ANN001
        """Read the file and project the anchors."""
        data = json.loads(path.read_text(encoding="utf-8"))
        rivers = []
        names = {}
        for index, river in enumerate(data["rivers"]):
            anchors = []
            for anchor in river["anchors"]:
                x, y = to_3035(*anchor["lonlat"])
                anchors.append((float(x), float(y), float(anchor["width_m"])))
            rivers.append({"id": river["id"], "anchors": anchors})
            for name in river["names"]:
                names[normalise_name(name)] = index
        return cls(
            by_order={int(k): float(v) for k, v in data["strahler_width_m"].items()},
            exponent=float(data["upstream_exponent"]),
            search_m=float(data["search_km"]) * 1000.0,
            rivers=rivers,
            names=names,
        )

    def order_width(self, orders: np.ndarray) -> np.ndarray:
        """Default width of each Strahler order."""
        top = max(self.by_order)
        return np.array(
            [self.by_order.get(int(min(max(o, 1), top)), 1.5) for o in orders]
        )

    def river_widths(self, name: str, points: np.ndarray) -> np.ndarray | None:
        """Anchored widths along a named stroke (``None`` if not an anchored river)."""
        index = self.names.get(normalise_name(name))
        if index is None or len(points) < 2:
            return None
        chain = vs.chainage(points)
        found = []
        for x, y, width in self.rivers[index]["anchors"]:
            dist = np.hypot(points[:, 0] - x, points[:, 1] - y)
            nearest = int(np.argmin(dist))
            if dist[nearest] <= self.search_m:
                found.append((chain[nearest], width))
        if not found:
            return None
        found.sort()
        at = np.array([c for c, _ in found])
        logw = np.log([w for _, w in found])
        widths = np.exp(np.interp(chain, at, logw))
        first_chain, first_width = found[0]
        upstream = chain < first_chain
        if first_chain > 0 and upstream.any():
            ratio = np.maximum(chain[upstream], 1.0) / first_chain
            widths[upstream] = first_width * ratio**self.exponent
        return widths


def normalise_name(name: str) -> str:
    """Lower-case name without article (``la Seine`` -> ``seine``)."""
    text = name.strip().lower().replace("’", "'")
    for prefix in ("fleuve ", "river ", "rivière "):
        text = text.removeprefix(prefix)
    for article in ("la ", "le ", "les ", "l'"):
        if text.startswith(article):
            text = text[len(article) :]
            break
    return text.strip()


def stroke_widths(
    model: WidthModel,
    name: str,
    points: np.ndarray,
    orders: np.ndarray,
    width_min: np.ndarray,
    width_max: np.ndarray,
) -> np.ndarray:
    """Width per vertex: anchors, else Strahler class, bounded by the source class."""
    base = model.order_width(orders)
    anchored = model.river_widths(name, points)
    if anchored is not None:
        widths = np.maximum(anchored, base * 0.5)
    else:
        lo = np.where(np.isfinite(width_min), width_min, 0.0)
        hi = np.where(np.isfinite(width_max), width_max, np.inf)
        widths = np.clip(base, lo, hi)
    # Never narrower downstream.
    return np.maximum.accumulate(widths)


# ---------------------------------------------------------------- the network


def lookup_links(stroke: hydro_sources.Stroke, along: np.ndarray) -> np.ndarray:
    """Link index of each resampled vertex (by chainage along the source stroke)."""
    chain = vs.chainage(stroke.points)
    index = np.clip(np.searchsorted(chain, along, side="right") - 1, 0, len(chain) - 1)
    return stroke.link_of_point[index]


def receivers(
    table: hydro_sources.LinkTable, strokes: list[hydro_sources.Stroke]
) -> np.ndarray:
    """Index of the stroke each stroke flows into (-1 at a mouth or border)."""
    stroke_of_link = np.full(len(table), -1, dtype=np.int64)
    for s, stroke in enumerate(strokes):
        stroke_of_link[stroke.links] = s
    outgoing: dict[int, list[int]] = defaultdict(list)
    for i in range(len(table)):
        outgoing[int(table.start[i])].append(i)
    result = np.full(len(strokes), -1, dtype=np.int64)
    for s, stroke in enumerate(strokes):
        last = stroke.links[-1]
        for nxt in outgoing.get(int(table.end[last]), []):
            target = int(stroke_of_link[nxt])
            if target >= 0 and target != s:
                result[s] = target
                break
    return result


def downstream_first(receiver: np.ndarray) -> list[int]:
    """Stroke indices with every receiver before its tributaries."""
    children: dict[int, list[int]] = defaultdict(list)
    roots = []
    for s, r in enumerate(receiver):
        if r < 0:
            roots.append(s)
        else:
            children[int(r)].append(s)
    order, seen = [], set()
    stack = list(reversed(roots))
    while stack:
        s = stack.pop()
        if s in seen:
            continue
        seen.add(s)
        order.append(s)
        stack.extend(children.get(s, []))
    order.extend(s for s in range(len(receiver)) if s not in seen)  # cycles
    return order


def join_confluences(lines: list[RiverLine]) -> None:
    """Level caps and geometric joins at confluences, receivers first.

    The receiver's level at the confluence is final when its tributary is
    processed: the tributary's tail is raised to it if needed (water cannot flow
    uphill into the receiver), and its last vertices are blended onto the
    receiver's nearest vertex.
    """
    receiver = np.array([line.receiver for line in lines], dtype=np.int64)
    for s in downstream_first(receiver):
        line = lines[s]
        line.level = vs.isotonic_decreasing(line.level)
        r = line.receiver
        if r < 0 or len(line.points) < 2 or len(lines[r].points) < 2:
            continue
        target = lines[r]
        dist = np.hypot(
            target.points[:, 0] - line.points[-1, 0],
            target.points[:, 1] - line.points[-1, 1],
        )
        k = int(np.argmin(dist))
        if dist[k] > 2000.0:  # not really joined (cut by a filter): leave it
            continue
        join_level = float(target.level[k])
        line.level = np.maximum(line.level, join_level)
        count = min(JOIN_BLEND_VERTICES, len(line.points) - 1)
        shift = target.points[k] - line.points[-1]
        weights = np.linspace(0.0, 1.0, count + 1)[:, None]
        line.points[-count - 1 :] = line.points[-count - 1 :] + shift * weights


# ---------------------------------------------------------------- zones/flags


@dataclass
class ZoneFlags:
    """Polygons of the notes file, projected, with their CAFV flags."""

    polygons: list[tuple[shapely.Polygon, int]]

    @classmethod
    def load(cls, notes: dict, to_3035) -> ZoneFlags:  # noqa: ANN001
        """Project the zones of ``historical_hydro_notes.json``."""
        names = {
            "divagating": fine_tiles.FLAG_DIVAGATING,
            "tidal": fine_tiles.FLAG_TIDAL,
            "rectified": fine_tiles.FLAG_RECTIFIED,
            "wetland": fine_tiles.FLAG_WETLAND,
        }
        polygons = []
        for zone in notes["zones"]:
            ring = np.array(zone["polygon_lonlat"], dtype=np.float64)
            x, y = to_3035(ring[:, 0], ring[:, 1])
            flags = 0
            for flag in zone["flags"]:
                flags |= names[flag]
            polygons.append((shapely.Polygon(np.column_stack([x, y])), flags))
        return cls(polygons)

    def per_vertex(self, points: np.ndarray) -> np.ndarray:
        """Bit field of zone flags at each vertex."""
        flags = np.zeros(len(points), dtype=np.int64)
        for polygon, bits in self.polygons:
            minx, miny, maxx, maxy = polygon.bounds
            near = (
                (points[:, 0] >= minx)
                & (points[:, 0] <= maxx)
                & (points[:, 1] >= miny)
                & (points[:, 1] <= maxy)
            )
            if near.any():
                inside = np.zeros(len(points), dtype=bool)
                inside[near] = shapely.contains_xy(
                    polygon, points[near, 0], points[near, 1]
                )
                flags[inside] |= bits
        return flags


# -------------------------------------------------------------------- tiling


def simplify_tolerance(relief: fine_relief.FineRelief, points: np.ndarray) -> float:
    """Douglas-Peucker tolerance from the finest level along a line."""
    levels = relief.finest_level(points[:, 0], points[:, 1])
    finest = int(levels.max()) if len(levels) else 0
    return float(np.clip(0.35 * relief.pixel_m(max(finest, 0)), 1.5, 30.0))


def line_flags(line: RiverLine, zone_bits: np.ndarray, order: int) -> int:
    """CAFV flags of a (piece of) line: zones, tidal, intermittent, order."""
    flags = 0
    for bit in (
        fine_tiles.FLAG_DIVAGATING,
        fine_tiles.FLAG_RECTIFIED,
        fine_tiles.FLAG_WETLAND,
        fine_tiles.FLAG_TIDAL,
    ):
        if np.mean((zone_bits & bit) > 0) >= 0.5:
            flags |= bit
    if np.mean(line.tidal) >= 0.5:
        flags |= fine_tiles.FLAG_TIDAL
    if np.mean(line.intermittent) >= 0.5:
        flags |= fine_tiles.FLAG_INTERMITTENT
    if line.source == "naturalearth":
        flags |= fine_tiles.FLAG_COARSE
    flags |= (min(max(order, 0), 15) & 0xF) << 24
    flags |= (SOURCE_CODES[line.source] & 0xF) << 28
    return flags


def tile_lines(
    lines: list[RiverLine],
    relief: fine_relief.FineRelief,
    zones: ZoneFlags,
    bounds: tuple[float, float, float, float],
) -> dict[tuple[int, int], fine_tiles.TileLines]:
    """Simplify, convert to world units and cut every line along the E2 tiles."""
    mpp = (bounds[2] - bounds[0]) / 4096.0
    side = fine_tiles.tile_units(fine_tiles.TILE_LEVEL)
    tiles: dict[tuple[int, int], fine_tiles.TileLines] = defaultdict(
        fine_tiles.TileLines
    )
    for feature, line in enumerate(lines):
        if len(line.points) < 2:
            continue
        keep = vs.simplify_indices(line.points, simplify_tolerance(relief, line.points))
        pts = line.points[keep]
        units = np.column_stack(
            [(pts[:, 0] - bounds[0]) / mpp, (bounds[3] - pts[:, 1]) / mpp]
        )
        bits = zones.per_vertex(pts)
        attrs = np.column_stack(
            [
                line.level[keep],
                line.width[keep],
                bits.astype(np.float64),
                np.arange(len(keep)),
            ]
        )
        for col, row, xy, att in fine_tiles.split_by_tiles(units, attrs, side):
            piece_bits = att[:, 2].astype(np.int64)
            tiles[(col, row)].add(
                feature,
                line_flags(line, piece_bits, line.order),
                xy,
                att[:, 0],
                att[:, 1],
            )
    return dict(tiles)


# ----------------------------------------------------------------------- build


@dataclass
class HydroResult:
    """Summary of ``geo hydro-fine``."""

    manifest: Path
    lines: int
    points: int
    tiles: int
    total_bytes: int
    per_source_km: dict[str, float]
    seconds: float


def _transformer():  # noqa: ANN202
    return Transformer.from_crs("EPSG:4326", "EPSG:3035", always_xy=True)


def build(
    map_dir: Path = MAP_DIR,
    workers: int | None = None,
    sources: tuple[str, ...] = ("topage", "osor", "euhydro", "naturalearth"),
    log=print,  # noqa: ANN001
) -> HydroResult:
    """Run the whole pipeline (idempotent, resumable through the caches)."""
    started = time.time()
    workers = workers or max(1, (os.cpu_count() or 2) - 1)
    bounds = pyramid.map_bounds(map_dir)
    relief = fine_relief.FineRelief(map_dir, bounds)
    notes = json.loads((map_dir / NOTES_FILE).read_text(encoding="utf-8"))
    canal_filter = hydro_sources.CanalFilter.from_notes(notes)
    transformer = _transformer()
    zones = ZoneFlags.load(notes, transformer.transform)
    widths = WidthModel.load(map_dir / WIDTHS_FILE, transformer.transform)

    preparers = {
        "topage": lambda: prepare_topage(),
        "osor": lambda: prepare_osor(),
        "euhydro": lambda: prepare_euhydro(),
        "naturalearth": lambda: prepare_naturalearth(map_dir),
    }
    tables = {}
    for source in sources:
        log(f"source {source} …")
        tables[source] = hydro_sources.LinkTable.load(preparers[source]())

    national = [tables[s] for s in ("topage", "osor") if s in tables]
    raster = coverage_raster(national, bounds)

    lines: list[RiverLine] = []
    per_source_km: dict[str, float] = {}
    for source, table in tables.items():
        mids = midpoints(table)
        in_core = core_mask(relief, mids)
        if source in ("topage", "osor"):
            keep = in_core
        elif source == "euhydro":
            keep = in_core & ~covered(raster, bounds, mids)
        else:  # Natural Earth: off the core only
            keep = ~in_core
        selected = select_links(table, canal_filter, keep)
        if source == "naturalearth":
            selected = orient_by_relief(selected, relief)
        strokes = hydro_sources.build_strokes(selected)
        strokes = [s for s in strokes if vs.polyline_length(s.points) >= MIN_STROKE_M]
        orders = [_stroke_order(selected, s) for s in strokes]
        results = snap_all(source, strokes, orders, map_dir, bounds, workers, log)
        receiver = receivers(selected, strokes) if source != "naturalearth" else None
        base = len(lines)
        for s, (stroke, (xy, floor, along)) in enumerate(
            zip(strokes, results, strict=True)
        ):
            links = lookup_links(stroke, along)
            name = _stroke_name(selected, stroke)
            link_orders = selected.strahler[links]
            lines.append(
                RiverLine(
                    source=source,
                    name=name,
                    order=orders[s],
                    points=xy,
                    level=floor,
                    width=stroke_widths(
                        widths,
                        name,
                        xy,
                        link_orders,
                        selected.width_min[links],
                        selected.width_max[links],
                    ),
                    intermittent=selected.intermittent[links],
                    tidal=selected.tidal[links],
                    receiver=(base + int(receiver[s]))
                    if receiver is not None and receiver[s] >= 0
                    else -1,
                    orders=link_orders,
                )
            )
        per_source_km[source] = round(
            sum(vs.polyline_length(line.points) for line in lines[base:]) / 1000.0, 1
        )
        log(f"  {source} : {len(lines) - base} lignes, {per_source_km[source]:.0f} km")

    join_confluences(lines)
    tiles = tile_lines(lines, relief, zones, bounds)
    tiles_dir = map_dir / pyramid.PYRAMID_DIR_NAME / TILES_SUBDIR
    index = fine_tiles.write_tiles(
        tiles_dir / f"E{fine_tiles.TILE_LEVEL}",
        fine_tiles.LAYER_RIVERS,
        fine_tiles.TILE_LEVEL,
        tiles,
    )
    features = [
        {
            "name": line.name,
            "source": line.source,
            "order": line.order,
            "receiver": line.receiver,
            "length_km": round(vs.polyline_length(line.points) / 1000.0, 2),
        }
        for line in lines
    ]
    (tiles_dir / FEATURES_FILE).write_text(
        json.dumps({"features": features}, ensure_ascii=False, separators=(",", ":")),
        encoding="utf-8",
    )
    manifest_path = map_dir / MANIFEST_FILE
    write_manifest(manifest_path, index, lines, per_source_km, tiles_dir)
    return HydroResult(
        manifest=manifest_path,
        lines=len(lines),
        points=sum(entry["points"] for entry in index),
        tiles=len(index),
        total_bytes=sum(entry["bytes"] for entry in index),
        per_source_km=per_source_km,
        seconds=time.time() - started,
    )


def _stroke_name(table: hydro_sources.LinkTable, stroke: hydro_sources.Stroke) -> str:
    """Name carried by most of the stroke's length (downstream links weigh more)."""
    weights: dict[str, float] = defaultdict(float)
    for rank, link in enumerate(stroke.links):
        name = str(table.name[link])
        if name:
            weights[name] += 1.0 + rank
    return max(weights, key=weights.get) if weights else ""


def orient_by_relief(
    table: hydro_sources.LinkTable, relief: fine_relief.FineRelief
) -> hydro_sources.LinkTable:
    """Reverse links whose start is lower than their end (no flow direction)."""
    for i, line in enumerate(table.lines):
        ends = relief.sample(line[[0, -1], 0], line[[0, -1], 1])
        if np.all(np.isfinite(ends)) and ends[0] < ends[1]:
            table.lines[i] = line[::-1].copy()
            table.start[i], table.end[i] = table.end[i], table.start[i]
    return table


def write_manifest(
    path: Path,
    index: list[dict],
    lines: list[RiverLine],
    per_source_km: dict[str, float],
    tiles_dir: Path,
) -> None:
    """Versioned manifest ``data/map/rivers_fine.json``."""
    manifest = {
        "description": "Réseau hydrographique fin recalé sur la pyramide de relief (lot ZG5a, ADR 0036). Généré par `cent-ans geo hydro-fine` : ne pas modifier à la main. Tuiles binaires CAFV hors git dans `dir` (voir docs/geo.md, section « Hydrographie fine »). Rendu seulement, aucune règle de jeu.",
        "version": 1,
        "format": "CAFV",
        "format_version": fine_tiles.VERSION,
        "layer": fine_tiles.LAYER_RIVERS,
        "tile_level": fine_tiles.TILE_LEVEL,
        "tile_units": fine_tiles.tile_units(fine_tiles.TILE_LEVEL),
        "dir": f"{pyramid.PYRAMID_DIR_NAME}/{TILES_SUBDIR}",
        "pattern": f"E{fine_tiles.TILE_LEVEL}/{{col}}_{{row}}.bin",
        "features_file": f"{pyramid.PYRAMID_DIR_NAME}/{TILES_SUBDIR}/{FEATURES_FILE}",
        "coordinates": "unités monde (pixels carte 4096), origine nord-ouest",
        "z": "niveau d'eau en mètres (hauteurs de rendu de la pyramide), non exagéré",
        "width": "largeur du lit mouillé en mètres",
        "flags": {
            "divagating": fine_tiles.FLAG_DIVAGATING,
            "tidal": fine_tiles.FLAG_TIDAL,
            "intermittent": fine_tiles.FLAG_INTERMITTENT,
            "rectified": fine_tiles.FLAG_RECTIFIED,
            "coarse": fine_tiles.FLAG_COARSE,
            "wetland": fine_tiles.FLAG_WETLAND,
            "order_shift": 24,
            "source_shift": 28,
        },
        "sources": {
            "topage": "BD TOPAGE® 2025 (IGN, OFB, SANDRE) — Licence Ouverte 2.0",
            "osor": "OS Open Rivers — Contains OS data © Crown copyright and database right, Open Government Licence v3",
            "euhydro": "Copernicus Land Monitoring Service, EU-Hydro River Network Database v1.3 (EEA)",
            "naturalearth": "Natural Earth 10m (domaine public), hors cœur",
        },
        "source_codes": SOURCE_CODES,
        "stats": {
            "lines": len(lines),
            "km_by_source": per_source_km,
            "tiles": len(index),
            "points": sum(entry["points"] for entry in index),
            "total_bytes": sum(entry["bytes"] for entry in index),
        },
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat(),
        "tiles": index,
    }
    text = json.dumps(manifest, ensure_ascii=False, indent=1)
    path.write_text(text + "\n", encoding="utf-8")
    del tiles_dir
