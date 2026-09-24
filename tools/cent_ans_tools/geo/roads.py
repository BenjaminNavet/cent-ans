"""Roads -> ``data/map/roads.geojson`` (lot C3, spec 2026-09-24 § 3.4).

Primary source: **Itiner-e** (de Soto, Pažout, Brughmans et al., *A High-Resolution
Dataset of Roads of the Roman Empire*, static version 2024, v1.3, Zenodo
doi:10.5281/zenodo.17122148, CC BY 4.0), downloaded without an account from
the Zenodo API as a GeoPackage (EPSG:3395). ``Hypothetical`` segments are
dropped, ``Certain`` and ``Conjectured`` kept; lines are reprojected into map
pixels and kept when they run through a playable province.

Itiner-e stops at the Roman frontier (no Ireland, Scotland beyond the Forth,
Germania east of the Rhine, Denmark, Sweden, Bohemia). Provinces it does not
cover receive **computed roads**: least-cost paths (slope from
``heightmap.png``, river crossings, no sea) along the land edges of the
settlement graph between ``city`` and ``town`` settlements. The same
computation is the full fallback when Itiner-e cannot be downloaded (or with
``computed=True``).

Output: GeoJSON ``FeatureCollection`` without CRS, coordinates in map pixels
(1 decimal), properties ``name``, ``type`` (``main`` / ``secondary`` /
``computed``), ``certainty`` and ``source`` (``itiner-e`` / ``computed``).
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass
from pathlib import Path

import geopandas as gpd
import numpy as np
import shapely
import shapely.affinity
from rasterio.features import rasterize
from shapely.geometry import LineString, MultiLineString, box, mapping
from skimage.graph import MCP_Geometric

from cent_ans_tools.geo import download, settlements, terrain
from cent_ans_tools.geo.project import CRS_MAP, MapGrid

MAP_DIR = settlements.MAP_DIR
ROADS_FILE = settlements.ROADS_FILE

ITINERE_RECORD = "17122148"
ITINERE_FILE = "itinere_roads.gpkg"
ITINERE_URL = (
    f"https://zenodo.org/api/records/{ITINERE_RECORD}/files/{ITINERE_FILE}/content"
)
DOWNLOAD_ATTEMPTS = 3
ITINERE_TYPES = {"Main Road": "main", "Secondary Road": "secondary"}
DROPPED_CERTAINTY = {"Hypothetical"}
SIMPLIFY_PX = 0.4
MIN_PLAYABLE_FRACTION = 0.5
# A province is "covered" by Itiner-e when road pixels reach this density
# (road pixels per 1,000 province pixels).
COVERED_ROAD_DENSITY = 4.0

# Least-cost fallback, solved on a grid of SIZE_PX / WORK_FACTOR.
WORK_FACTOR = 2
SLOPE_COST = 25.0  # extra cost per unit of slope (rise / run)
RIVER_COST = 6.0  # extra cost on a major river pixel (crossing or following)
RIVER_MAX_SCALERANK = 6
WINDOW_MARGIN_PX = 40  # work-grid pixels around the edge's bounding box
ROAD_KINDS = {"city", "town"}


@dataclass
class RoadResult:
    """Output of :func:`build`."""

    path: Path
    source: str
    features: int
    km: float
    seconds: float


def download_itinere(force: bool = False) -> Path:
    """Fetch the Itiner-e GeoPackage (cached), retrying a few times."""
    target = download.RAW_DIR / "itinere" / ITINERE_FILE
    last_error: Exception | None = None
    for _ in range(DOWNLOAD_ATTEMPTS):
        try:
            return download.download_file(ITINERE_URL, target, force)
        except download.DownloadError as error:
            last_error = error
    raise download.DownloadError(f"Itiner-e indisponible : {last_error}")


def _to_pixels(geometry, grid: MapGrid):  # noqa: ANN001, ANN202
    """Projected geometry -> map pixel geometry."""
    minx, _, _, maxy = grid.bounds
    mpp = grid.meters_per_px
    return shapely.transform(
        geometry,
        lambda xy: np.column_stack([(xy[:, 0] - minx) / mpp, (maxy - xy[:, 1]) / mpp]),
    )


def _lines(geometry) -> list[LineString]:  # noqa: ANN001
    if geometry.is_empty:
        return []
    if isinstance(geometry, LineString):
        return [geometry]
    if isinstance(geometry, MultiLineString):
        return list(geometry.geoms)
    return [g for part in getattr(geometry, "geoms", []) for g in _lines(part)]


def itinere_features(gpkg: Path, grid: MapGrid, labels: np.ndarray) -> list[dict]:
    """Itiner-e segments in map pixels, restricted to playable provinces."""
    frame = gpd.read_file(gpkg)
    frame = frame[~frame["Segment_s"].isin(DROPPED_CERTAINTY)]
    frame = frame.to_crs(CRS_MAP)
    frame = frame[frame.intersects(box(*grid.bounds))]
    frame = frame.assign(geometry=frame.geometry.clip(box(*grid.bounds)))
    size = labels.shape[0]
    features = []
    for row in frame.itertuples():
        pixel_geometry = _to_pixels(row.geometry, grid).simplify(SIMPLIFY_PX)
        for line in _lines(pixel_geometry):
            if line.length < 1.0:
                continue
            coords = np.asarray(line.coords)
            cols = np.clip(coords[:, 0].astype(int), 0, size - 1)
            rows = np.clip(coords[:, 1].astype(int), 0, size - 1)
            if (labels[rows, cols] > 0).mean() < MIN_PLAYABLE_FRACTION:
                continue
            features.append(
                {
                    "type": "Feature",
                    "properties": {
                        "name": row.Name if isinstance(row.Name, str) else None,
                        "type": ITINERE_TYPES.get(row.Type, "secondary"),
                        "certainty": str(row.Segment_s).lower(),
                        "source": "itiner-e",
                    },
                    "geometry": mapping(line),
                }
            )
    return features


def covered_provinces(features: list[dict], labels: np.ndarray) -> set[int]:
    """Province indices whose Itiner-e road density reaches :data:`COVERED_ROAD_DENSITY`."""
    if not features:
        return set()
    raster = rasterize(
        [(shapely.geometry.shape(f["geometry"]), 1) for f in features],
        out_shape=labels.shape,
        fill=0,
        dtype=np.uint8,
    )
    count = int(labels.max()) + 1
    road_px = np.bincount(labels[raster > 0].ravel(), minlength=count)
    area_px = np.bincount(labels.ravel(), minlength=count)
    density = 1000.0 * road_px / np.maximum(area_px, 1)
    return {int(i) for i in np.nonzero(density >= COVERED_ROAD_DENSITY)[0] if i > 0}


def cost_surface(map_dir: Path, grid: MapGrid, labels: np.ndarray) -> np.ndarray:
    """Work-grid travel cost: 1 + slope + river penalty, ``inf`` outside provinces."""
    heights = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    size = grid.size_px // WORK_FACTOR
    small = heights.reshape(size, WORK_FACTOR, size, WORK_FACTOR).mean(axis=(1, 3))
    dy, dx = np.gradient(small, grid.meters_per_px * WORK_FACTOR)
    cost = 1.0 + SLOPE_COST * np.hypot(dx, dy)
    rivers = json.loads((map_dir / "rivers.geojson").read_text(encoding="utf-8"))
    majors = [
        (shapely.geometry.shape(f["geometry"]), 1)
        for f in rivers["features"]
        if (f["properties"].get("scalerank") or 99) <= RIVER_MAX_SCALERANK
    ]
    if majors:
        scale = 1.0 / WORK_FACTOR
        river = rasterize(
            [
                (shapely.affinity.scale(g, scale, scale, origin=(0, 0)), v)
                for g, v in majors
            ],
            out_shape=(size, size),
            fill=0,
            dtype=np.uint8,
        )
        cost = cost + RIVER_COST * river
    playable = labels[::WORK_FACTOR, ::WORK_FACTOR] > 0
    return np.where(playable, cost, np.inf)


def least_cost_path(
    cost: np.ndarray, start: tuple[float, float], end: tuple[float, float]
) -> LineString | None:
    """Least-cost path between two map-pixel points, in map pixels (``None`` if unreachable)."""
    size = cost.shape[0]
    (sx, sy), (ex, ey) = (
        (int(p[0] / WORK_FACTOR), int(p[1] / WORK_FACTOR)) for p in (start, end)
    )
    top = max(0, min(sy, ey) - WINDOW_MARGIN_PX)
    left = max(0, min(sx, ex) - WINDOW_MARGIN_PX)
    bottom = min(size, max(sy, ey) + WINDOW_MARGIN_PX + 1)
    right = min(size, max(sx, ex) + WINDOW_MARGIN_PX + 1)
    window = cost[top:bottom, left:right].copy()
    window[sy - top, sx - left] = 1.0
    window[ey - top, ex - left] = 1.0
    graph = MCP_Geometric(window, fully_connected=True)
    costs, _ = graph.find_costs([(sy - top, sx - left)], [(ey - top, ex - left)])
    if not np.isfinite(costs[ey - top, ex - left]):
        return None
    cells = np.asarray(graph.traceback((ey - top, ex - left)), dtype=np.float64)
    xs = (cells[:, 1] + left + 0.5) * WORK_FACTOR
    ys = (cells[:, 0] + top + 0.5) * WORK_FACTOR
    coords = np.column_stack([xs, ys])
    coords[0], coords[-1] = start, end
    return LineString(coords).simplify(1.0)


def computed_features(
    map_dir: Path,
    grid: MapGrid,
    labels: np.ndarray,
    skip_provinces: set[int],
) -> list[dict]:
    """Least-cost roads along city/town land edges outside ``skip_provinces``."""
    places, edges, *_ = settlements.prepare(map_dir=map_dir)
    by_id = {s.id: s for s in places}
    geometry = settlements.load_province_geometry(map_dir)
    index_of = {pid: props["index"] for pid, props in geometry.items()}
    cost = cost_surface(map_dir, grid, labels)
    features = []
    for edge in edges:
        a, b = by_id[edge.a], by_id[edge.b]
        if edge.sea or a.kind not in ROAD_KINDS or b.kind not in ROAD_KINDS:
            continue
        if (
            index_of[a.province] in skip_provinces
            and index_of[b.province] in skip_provinces
        ):
            continue
        line = least_cost_path(cost, a.px, b.px)
        if line is None:
            continue
        features.append(
            {
                "type": "Feature",
                "properties": {
                    "name": f"{a.name} – {b.name}",
                    "type": "computed",
                    "certainty": "computed",
                    "source": "computed",
                },
                "geometry": mapping(line),
            }
        )
    return features


def _round(feature: dict) -> dict:
    coords = [[round(x, 1), round(y, 1)] for x, y in feature["geometry"]["coordinates"]]
    return {**feature, "geometry": {"type": "LineString", "coordinates": coords}}


def build(
    force: bool = False, computed: bool = False, map_dir: Path = MAP_DIR
) -> RoadResult:
    """Write ``roads.geojson`` from Itiner-e (plus computed complement) or the fallback.

    Args:
        force: Re-download Itiner-e.
        computed: Skip Itiner-e and compute every road.
        map_dir: ``data/map``.
    """
    started = time.perf_counter()
    grid = settlements.provinces_step.load_grid(map_dir)
    labels = settlements.load_labels(map_dir)
    features: list[dict] = []
    source = "computed"
    if not computed:
        try:
            features = itinere_features(download_itinere(force), grid, labels)
            source = "itiner-e + computed"
        except download.DownloadError:
            features = []
    features += computed_features(
        map_dir, grid, labels, covered_provinces(features, labels)
    )
    features = [_round(f) for f in features]
    path = map_dir / ROADS_FILE
    path.write_text(
        json.dumps(
            {"type": "FeatureCollection", "features": features}, ensure_ascii=False
        )
        + "\n",
        encoding="utf-8",
    )
    km = (
        sum(shapely.geometry.shape(f["geometry"]).length for f in features)
        * grid.meters_per_px
        / 1000.0
    )
    return RoadResult(path, source, len(features), km, time.perf_counter() - started)
