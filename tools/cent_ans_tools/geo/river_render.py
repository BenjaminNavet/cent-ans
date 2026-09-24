"""Offline river data for the campaign map renderer (lot V4, A1-11).

Reads ``rivers.geojson``, ``crossings.json`` and ``river_styles.json`` in ``data/map/`` and writes:

``rivers_render.json``
    River polylines oriented downstream (elevation at the ends), smoothed (Chaikin), with a
    displayed width per point: the per-river mouth width of ``river_styles.json`` (or a width by
    importance) narrowed upstream by elevation. Custom zones (``custom_zones``) are cut out.
``river_bed.png``
    L8, full map resolution: signed distance to the river bank in map pixels,
    ``(value - 128) / RIVER_BED_SCALE`` (negative inside the bed). Read by ``terrain.gdshader`` for
    the carved bed, the banks and by the vegetation (no trees in the water).
``crossings_px.json``
    Bridges, fords and ferries of ``crossings.json`` projected in map pixels and snapped onto the
    displayed river: position, river direction, displayed width, structure (stone, wood, boats,
    ferry, ford). Passes are left out (nothing to draw on the river).

Rendering data only: no game rule depends on these files.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.geo import download, terrain
from cent_ans_tools.geo.navgrid import MAJOR_RIVERS, RIVER_FR
from cent_ans_tools.geo.provinces import load_grid

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"

STYLES_FILE = "river_styles.json"
RENDER_FILE = "rivers_render.json"
BED_FILE = "river_bed.png"
CROSSINGS_FILE = "crossings.json"
CROSSINGS_PX_FILE = "crossings_px.json"

RIVER_BED_SCALE = 16.0  # 1 px = 16 levels, signed range ±8 px
BED_RANGE_PX = 127.0 / RIVER_BED_SCALE
SMOOTH_ITERATIONS = 2
SNAP_NAMED_PX = 14.0  # search radius for the named river around a crossing
SNAP_ANY_PX = 6.0  # fallback: any displayed river of importance >= 3
DEFAULT_STRUCTURE = {"bridge": "stone", "ford": "ford"}
ROAD_SNAP_PX = (
    4.0  # a historic crossing moves onto the nearest road crossing within this radius
)
ROAD_AXIS_PX = (
    2.5  # otherwise it takes the direction of a road ending within this radius
)
ROAD_MERGE_PX = 0.9  # overlapping roads: crossings closer than this are merged
ROAD_BRIDGE_MIN_WIDTH = 0.2  # narrower streams crossed by a road get no bridge
STREAM_WIDTH = 0.3  # generic road bridge: stone culvert below, timber bridge above


@dataclass(frozen=True)
class RiverRenderResult:
    """Paths written by :func:`build` and a few counts for the report."""

    render: Path
    bed: Path
    crossings: Path
    rivers: int
    points: int
    snapped: int
    unsnapped: list[str]


def importance_of(props: dict) -> int:
    """Same rule as ``MapData._load_rivers``: Strahler order, else ``12 - scalerank``."""
    if "strahler" in props:
        return int(np.clip(int(props["strahler"]), 1, 6))
    return int(np.clip(12 - int(props.get("scalerank", 10)), 0, 6))


def linestrings(geometry: dict) -> list[list[list[float]]]:
    """LineString / MultiLineString coordinates as a list of lines."""
    kind = geometry.get("type", "")
    coords = geometry.get("coordinates", [])
    if kind == "LineString":
        return [coords]
    if kind == "MultiLineString":
        return list(coords)
    return []


def chaikin(points: np.ndarray, iterations: int) -> np.ndarray:
    """Chaikin corner cutting, keeping both end points."""
    for _ in range(iterations):
        if len(points) < 3:
            return points
        a = points[:-1]
        b = points[1:]
        q = 0.75 * a + 0.25 * b
        r = 0.25 * a + 0.75 * b
        middle = np.empty((2 * len(a), 2), dtype=points.dtype)
        middle[0::2] = q
        middle[1::2] = r
        points = np.vstack([points[:1], middle[1:-1], points[-1:]])
    return points


def taper(height_m: np.ndarray, styles: dict) -> np.ndarray:
    """Width factor by elevation: 1 in the lowlands, ``taper_min`` in the mountains."""
    low = float(styles["taper_low_m"])
    high = float(styles["taper_high_m"])
    t = np.clip((height_m - low) / max(high - low, 1.0), 0.0, 1.0)
    t = t * t * (3.0 - 2.0 * t)
    return 1.0 - (1.0 - float(styles["taper_min"])) * t


def mouth_width(name: str | None, importance: int, styles: dict) -> float:
    """Displayed width at the mouth (map px)."""
    table: dict = styles["rivers"]
    if name and name in table:
        return float(table[name])
    return float(styles["width_by_importance"][int(np.clip(importance, 0, 6))])


def sample_bilinear(grid: np.ndarray, xs: np.ndarray, ys: np.ndarray) -> np.ndarray:
    """Bilinear sample of ``grid`` at continuous pixel coordinates (pixel centres at +0.5)."""
    h, w = grid.shape
    fx = np.clip(xs - 0.5, 0.0, w - 1.001)
    fy = np.clip(ys - 0.5, 0.0, h - 1.001)
    x0 = fx.astype(np.int64)
    y0 = fy.astype(np.int64)
    tx = fx - x0
    ty = fy - y0
    top = grid[y0, x0] * (1 - tx) + grid[y0, x0 + 1] * tx
    bottom = grid[y0 + 1, x0] * (1 - tx) + grid[y0 + 1, x0 + 1] * tx
    return top * (1 - ty) + bottom * ty


def in_zones(points: np.ndarray, zones: list[dict]) -> np.ndarray:
    """Mask of points inside any custom zone (``px``, ``radius_px``)."""
    inside = np.zeros(len(points), dtype=bool)
    for zone in zones:
        center = np.asarray(zone["px"], dtype=np.float64)
        inside |= np.hypot(*(points - center).T) < float(zone["radius_px"])
    return inside


def split_outside(
    points: np.ndarray, widths: np.ndarray, zones: list[dict]
) -> list[tuple[np.ndarray, np.ndarray]]:
    """Runs of consecutive points outside every zone (at least two points each)."""
    if not zones:
        return [(points, widths)]
    inside = in_zones(points, zones)
    runs: list[tuple[np.ndarray, np.ndarray]] = []
    start = None
    for i, flag in enumerate([*inside, True]):
        if not flag and start is None:
            start = i
        elif flag and start is not None:
            if i - start >= 2:
                runs.append((points[start:i], widths[start:i]))
            start = None
    return runs


def prepare_rivers(
    features: list[dict], height_m: np.ndarray, styles: dict, zones: list[dict]
) -> list[dict]:
    """Oriented, smoothed, width-annotated river polylines (map px)."""
    rivers: list[dict] = []
    for feature in features:
        props = feature.get("properties", {}) or {}
        name = props.get("name")
        importance = importance_of(props)
        for line in linestrings(feature.get("geometry", {}) or {}):
            points = np.asarray(line, dtype=np.float64)
            if len(points) < 2:
                continue
            ends = sample_bilinear(height_m, points[[0, -1], 0], points[[0, -1], 1])
            if ends[0] < ends[1]:
                points = points[::-1]  # the source is lower: flow goes the other way
            points = chaikin(points, SMOOTH_ITERATIONS)
            heights = sample_bilinear(height_m, points[:, 0], points[:, 1])
            widths = mouth_width(name, importance, styles) * taper(heights, styles)
            for run_points, run_widths in split_outside(points, widths, zones):
                rivers.append(
                    {
                        "name": name or "",
                        "importance": importance,
                        "points": run_points,
                        "widths": run_widths,
                    }
                )
    return rivers


def segment_distance(
    px: np.ndarray, py: np.ndarray, a: np.ndarray, b: np.ndarray
) -> tuple[np.ndarray, np.ndarray]:
    """Distance from pixel centres to segment ``ab`` and the projection parameter in [0, 1]."""
    ab = b - a
    length2 = max(float(ab @ ab), 1e-12)
    t = np.clip(((px - a[0]) * ab[0] + (py - a[1]) * ab[1]) / length2, 0.0, 1.0)
    dx = px - (a[0] + t * ab[0])
    dy = py - (a[1] + t * ab[1])
    return np.hypot(dx, dy), t


def compute_bed(rivers: list[dict], size: int) -> np.ndarray:
    """Signed distance (map px) to the nearest bank; negative inside a river bed."""
    bed = np.full((size, size), BED_RANGE_PX, dtype=np.float32)
    reach = BED_RANGE_PX + 1.0
    for river in rivers:
        points: np.ndarray = river["points"]
        widths: np.ndarray = river["widths"]
        for i in range(len(points) - 1):
            a = points[i]
            b = points[i + 1]
            x0 = max(int(math.floor(min(a[0], b[0]) - reach)), 0)
            x1 = min(int(math.ceil(max(a[0], b[0]) + reach)), size)
            y0 = max(int(math.floor(min(a[1], b[1]) - reach)), 0)
            y1 = min(int(math.ceil(max(a[1], b[1]) + reach)), size)
            if x0 >= x1 or y0 >= y1:
                continue
            ys, xs = np.mgrid[y0:y1, x0:x1]
            distance, t = segment_distance(xs + 0.5, ys + 0.5, a, b)
            half = 0.5 * (widths[i] * (1.0 - t) + widths[i + 1] * t)
            signed = (distance - half).astype(np.float32)
            window = bed[y0:y1, x0:x1]
            np.minimum(window, signed, out=window)
    return bed


def encode_bed(bed: np.ndarray) -> np.ndarray:
    """L8 encoding of :func:`compute_bed` (``128 + d × RIVER_BED_SCALE``)."""
    return np.clip(np.round(128.0 + bed * RIVER_BED_SCALE), 0, 255).astype(np.uint8)


def ne_names(river_fr: str | None) -> tuple[str, ...]:
    """Natural Earth names of a river named in French in ``crossings.json``."""
    key = RIVER_FR.get(river_fr or "")
    if key:
        return MAJOR_RIVERS.get(key, ())
    return (river_fr,) if river_fr else ()


def nearest_on_rivers(
    point: np.ndarray, rivers: list[dict], radius: float
) -> tuple[float, np.ndarray, np.ndarray, float] | None:
    """Closest point of ``rivers`` to ``point`` within ``radius``: (distance, position, unit direction, width)."""
    best = None
    for river in rivers:
        points: np.ndarray = river["points"]
        lo = points.min(axis=0) - radius
        hi = points.max(axis=0) + radius
        if np.any(point < lo) or np.any(point > hi):
            continue
        a = points[:-1]
        ab = points[1:] - a
        length2 = np.maximum((ab * ab).sum(axis=1), 1e-12)
        t = np.clip(((point - a) * ab).sum(axis=1) / length2, 0.0, 1.0)
        projected = a + ab * t[:, None]
        distance = np.hypot(*(projected - point).T)
        i = int(np.argmin(distance))
        if distance[i] <= radius and (best is None or distance[i] < best[0]):
            direction = ab[i] / math.sqrt(length2[i])
            widths: np.ndarray = river["widths"]
            width = float(widths[i] * (1.0 - t[i]) + widths[i + 1] * t[i])
            best = (float(distance[i]), projected[i], direction, width)
    return best


def place_crossings(
    entries: list[dict],
    rivers: list[dict],
    to_pixel,
    zones: list[dict],  # noqa: ANN001
) -> tuple[list[dict], list[str]]:
    """Bridges, fords and ferries in map px, snapped onto the displayed river."""
    placed: list[dict] = []
    unsnapped: list[str] = []
    for entry in entries:
        kind = entry["type"]
        if kind == "pass":
            continue
        px, py = to_pixel(*entry["lonlat"])
        point = np.array([float(px), float(py)])
        names = ne_names(entry.get("river"))
        named = [r for r in rivers if r["name"] in names]
        hit = nearest_on_rivers(point, named, SNAP_NAMED_PX) if named else None
        if hit is None:
            major = [r for r in rivers if r["importance"] >= 3]
            hit = nearest_on_rivers(point, major, SNAP_ANY_PX)
        structure = entry.get("structure", DEFAULT_STRUCTURE.get(kind, "stone"))
        record = {
            "id": entry["id"],
            "name": entry["name"],
            "type": kind,
            "structure": structure,
            "river": entry.get("river", ""),
        }
        if hit is None:
            unsnapped.append(entry["id"])
            record.update(
                px=[round(float(px), 2), round(float(py), 2)],
                dir=[1.0, 0.0],
                width=0.0,
                snapped=False,
            )
        else:
            _, position, direction, width = hit
            record.update(
                px=[round(float(position[0]), 3), round(float(position[1]), 3)],
                dir=[round(float(direction[0]), 4), round(float(direction[1]), 4)],
                width=round(width, 3),
                snapped=True,
            )
        # Source position (the snapped one lies outside the zone, where the river was cut).
        record["in_custom_zone"] = (
            bool(in_zones(point[None, :], zones)[0]) if zones else False
        )
        placed.append(record)
    return placed, unsnapped


def road_crossings(rivers: list[dict], roads: list[dict]) -> list[dict]:
    """Intersections of the displayed roads with the rivers (merged when roads overlap).

    Returns dicts with ``px``, ``dir`` (river, unit), ``axis`` (road, unit), ``width`` (length to
    span along the road, capped at twice the river width), ``river`` and ``road_type``.
    """
    from shapely import LineString, STRtree

    lines = [LineString(r["points"]) for r in rivers]
    tree = STRtree(lines)
    hits: list[dict] = []
    for road in roads:
        points = np.asarray(road["points"], dtype=np.float64)
        if len(points) < 2:
            continue
        road_line = LineString(points)
        for index in tree.query(road_line, predicate="intersects"):
            river = rivers[int(index)]
            geometry = road_line.intersection(lines[int(index)])
            for point in getattr(geometry, "geoms", [geometry]):
                if point.geom_type != "Point":
                    continue
                p = np.array([point.x, point.y])
                if any(np.hypot(*(h["p"] - p)) < ROAD_MERGE_PX for h in hits):
                    continue
                snapped = nearest_on_rivers(p, [river], 0.5)
                if snapped is None:
                    continue
                _, _, river_dir, width = snapped
                if width < ROAD_BRIDGE_MIN_WIDTH:
                    continue
                a = points[:-1]
                ab = points[1:] - a
                length2 = np.maximum((ab * ab).sum(axis=1), 1e-12)
                t = np.clip(((p - a) * ab).sum(axis=1) / length2, 0.0, 1.0)
                k = int(np.argmin(np.hypot(*(a + ab * t[:, None] - p).T)))
                axis = ab[k] / math.sqrt(length2[k])
                sine = abs(axis[0] * river_dir[1] - axis[1] * river_dir[0])
                hits.append(
                    {
                        "p": p,
                        "dir": river_dir,
                        "axis": axis,
                        "width": min(width / max(sine, 0.5), width * 2.0),
                        "river": river["name"],
                        "road_type": road.get("type", "secondary"),
                    }
                )
    return hits


def nearest_road_axis(
    p: np.ndarray, roads: list[dict], radius: float
) -> np.ndarray | None:
    """Direction of the closest road segment within ``radius`` of ``p`` (None if none)."""
    best, best_d = None, radius
    for road in roads:
        points = np.asarray(road["points"], dtype=np.float64)
        if (
            len(points) < 2
            or np.any(p < points.min(axis=0) - radius)
            or np.any(p > points.max(axis=0) + radius)
        ):
            continue
        a = points[:-1]
        ab = points[1:] - a
        length2 = np.maximum((ab * ab).sum(axis=1), 1e-12)
        t = np.clip(((p - a) * ab).sum(axis=1) / length2, 0.0, 1.0)
        distance = np.hypot(*(a + ab * t[:, None] - p).T)
        k = int(np.argmin(distance))
        if distance[k] < best_d:
            best, best_d = ab[k] / math.sqrt(length2[k]), float(distance[k])
    return best


def attach_roads(
    placed: list[dict], hits: list[dict], roads: list[dict] | None = None
) -> list[dict]:
    """Move historic bridges onto the nearest road crossing; the others become generic bridges.

    A historic bridge without a road crossing nearby (roads ending on both banks) still carries
    the direction of the closest road (``ROAD_AXIS_PX``).
    """
    used = set()
    for record in placed:
        if not record["snapped"] or record["type"] != "bridge":
            continue
        p = np.asarray(record["px"])
        best, best_d = None, ROAD_SNAP_PX
        for k, hit in enumerate(hits):
            d = float(np.hypot(*(hit["p"] - p)))
            if d < best_d and k not in used:
                best, best_d = k, d
        if best is None:
            axis = nearest_road_axis(p, roads or [], ROAD_AXIS_PX)
            if axis is not None:
                river_dir = np.asarray(record["dir"])
                sine = abs(axis[0] * river_dir[1] - axis[1] * river_dir[0])
                if sine > 0.5:
                    record["axis"] = [
                        round(float(axis[0]), 4),
                        round(float(axis[1]), 4),
                    ]
                    record["width"] = round(
                        min(record["width"] / sine, record["width"] * 2.0), 3
                    )
            continue
        used.add(best)
        hit = hits[best]
        record.update(
            px=[round(float(hit["p"][0]), 3), round(float(hit["p"][1]), 3)],
            dir=[round(float(hit["dir"][0]), 4), round(float(hit["dir"][1]), 4)],
            axis=[round(float(hit["axis"][0]), 4), round(float(hit["axis"][1]), 4)],
            width=round(float(hit["width"]), 3),
        )
    generic = []
    for k, hit in enumerate(hits):
        if k in used:
            continue
        generic.append(
            {
                "id": f"road_{len(generic)}",
                "name": "",
                "type": "road",
                "structure": "stone" if hit["width"] < STREAM_WIDTH else "wood",
                "river": hit["river"],
                "road_type": hit["road_type"],
                "px": [round(float(hit["p"][0]), 3), round(float(hit["p"][1]), 3)],
                "dir": [round(float(hit["dir"][0]), 4), round(float(hit["dir"][1]), 4)],
                "axis": [
                    round(float(hit["axis"][0]), 4),
                    round(float(hit["axis"][1]), 4),
                ],
                "width": round(float(hit["width"]), 3),
                "snapped": True,
                "in_custom_zone": False,
            }
        )
    return generic


def load_zones(styles: dict, to_pixel) -> list[dict]:  # noqa: ANN001
    """Custom zones of ``river_styles.json`` with their centre in map px."""
    zones = []
    for zone in styles.get("custom_zones", []):
        px, py = to_pixel(*zone["lonlat"])
        zones.append({**zone, "px": [round(float(px), 2), round(float(py), 2)]})
    return zones


def build(map_dir: Path = MAP_DIR) -> RiverRenderResult:
    """Write ``rivers_render.json``, ``river_bed.png`` and ``crossings_px.json``."""
    styles = json.loads((map_dir / STYLES_FILE).read_text(encoding="utf-8"))
    grid = load_grid(map_dir)

    def to_pixel(lon: float, lat: float) -> tuple[float, float]:
        px, py = grid.lonlat_to_pixel(lon, lat)
        return float(px), float(py)

    zones = load_zones(styles, to_pixel)
    height_m = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    features = json.loads((map_dir / "rivers.geojson").read_text(encoding="utf-8"))[
        "features"
    ]
    rivers = prepare_rivers(features, height_m.astype(np.float32), styles, zones)

    render_path = map_dir / RENDER_FILE
    payload = {
        "description": "Généré par `cent-ans geo rivers-render` (lot V4) : ne pas modifier à la main.",
        "bed_scale": RIVER_BED_SCALE,
        "bank_px": styles["bank_px"],
        "custom_zones": [
            {
                "id": z["id"],
                "name": z.get("name", z["id"]),
                "px": z["px"],
                "radius_px": z["radius_px"],
                "boundary_bridges": bool(z.get("boundary_bridges", True)),
            }
            for z in zones
        ],
        "rivers": [
            {
                "name": r["name"],
                "importance": r["importance"],
                "points": np.round(r["points"], 2).tolist(),
                "widths": np.round(r["widths"], 3).tolist(),
            }
            for r in rivers
        ],
    }
    render_path.write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")), encoding="utf-8"
    )

    bed_path = map_dir / BED_FILE
    Image.fromarray(encode_bed(compute_bed(rivers, height_m.shape[0])), mode="L").save(
        bed_path, optimize=True
    )

    entries = json.loads((map_dir / CROSSINGS_FILE).read_text(encoding="utf-8"))[
        "crossings"
    ]
    placed, unsnapped = place_crossings(entries, rivers, to_pixel, zones)
    roads = [
        {"type": f["properties"].get("type", "secondary"), "points": line}
        for f in json.loads((map_dir / "roads.geojson").read_text(encoding="utf-8"))[
            "features"
        ]
        for line in linestrings(f.get("geometry", {}) or {})
    ]
    placed += attach_roads(placed, road_crossings(rivers, roads), roads)
    crossings_path = map_dir / CROSSINGS_PX_FILE
    crossings_path.write_text(
        json.dumps(
            {
                "description": "Généré par `cent-ans geo rivers-render` (lot V4) depuis crossings.json : ponts, gués et bacs en pixels carte, recalés sur le fleuve affiché. Rendu seulement.",
                "crossings": placed,
            },
            ensure_ascii=False,
            indent=1,
        ),
        encoding="utf-8",
    )
    return RiverRenderResult(
        render=render_path,
        bed=bed_path,
        crossings=crossings_path,
        rivers=len(rivers),
        points=sum(len(r["points"]) for r in rivers),
        snapped=sum(1 for c in placed if c["snapped"]),
        unsnapped=unsnapped,
    )
