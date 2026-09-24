"""Navigation grid for free army movement (lot M1, spec 2026-09-24-mouvement-libre § 2).

Output: ``data/map/navgrid.png``, 8-bit greyscale, :data:`NAVGRID_SIZE`² cells,
one cell = 2 × 2 map pixels (about 1.44 km). The value of a cell is the cost
of entering it (10 = one plain cell), :data:`IMPASSABLE` (255) = cannot be
entered. ``map.json`` gains ``"navgrid": {"size_px", "file", "scale"}``.

Layers, in order (costs from ``data/movement/rules.json``):

1. **Terrain**: the most expensive class that applies to the cell among
   plains, hills (altitude or slope), forest (``splat.png`` forest weight),
   marsh (flat lowland of a ``marsh`` province) and mountains (altitude,
   slope or rock weight). Slopes above ``slope_impassable_threshold`` are
   impassable.
2. **Minor rivers** (every ``rivers.geojson`` line that is not a major river):
   ``+ minor_river_extra``.
3. **Roads** (``roads.geojson``, 1 cell wide): ``× road_cost_factor``.
4. **Major rivers** (:data:`MAJOR_RIVERS`): impassable. Rasterised with
   ``all_touched`` and closed diagonally (:func:`four_connected`) so that an
   8-neighbour move can never slip between two diagonal river cells.
5. **Crossings** that reopen the major rivers: bridges and fords of
   ``crossings.json`` (river cells within :data:`CROSSING_RADIUS` of the
   nearest cell of the named river) and crossings of an Itiner-e road
   (short overlaps of the road raster with a major river).
6. **Passes** of ``crossings.json``: the least-cost path along the route
   points ignores the slope limit and is forced passable (mountain cost).
7. **Water** (sea at or below 0 m, lakes of the land mask): impassable.
8. **Settlements**: the cell of every settlement costs a plain.

Checks (:func:`check`): every settlement reaches every settlement of its land
mass through passable cells (8 neighbours); land masses with settlements but
no port are reported. A colour preview goes to ``docs/img/navgrid-preview.png``.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import shapely
import shapely.affinity
from PIL import Image, ImageDraw
from rasterio.features import rasterize
from scipy import ndimage
from skimage.graph import MCP_Geometric

from cent_ans_tools.geo import download, settlements, splat, terrain
from cent_ans_tools.geo.project import MapGrid

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = settlements.MAP_DIR
RULES_PATH = REPO_DIR / "data" / "movement" / "rules.json"
PREVIEW_PATH = REPO_DIR / "docs" / "img" / "navgrid-preview.png"
NAVGRID_FILE = "navgrid.png"
CROSSINGS_FILE = "crossings.json"
NAVGRID_SIZE = 2048
IMPASSABLE = 255
MAX_COST = 254

# Major rivers: key -> names used in rivers.geojson (Natural Earth, local names).
MAJOR_RIVERS: dict[str, tuple[str, ...]] = {
    "loire": ("Loire",),
    "seine": ("Seine",),
    "rhone": ("Rhône",),
    "garonne": ("Garonne",),
    "rhine": ("Rhein", "Rhin", "Rhine"),
    "thames": ("Thames",),
    "meuse": ("Maas",),
    "scheldt": ("Schelde",),
    "dordogne": ("Dordogne",),
    "saone": ("Saône",),
    "po": ("Po",),
    "tagus": ("Tajo", "Tejo"),
    "ebro": ("Ebro",),
    "danube": ("Danube", "Donau"),
}
# French names used by crossings.json -> major river key.
RIVER_FR: dict[str, str] = {
    "Loire": "loire",
    "Seine": "seine",
    "Rhône": "rhone",
    "Garonne": "garonne",
    "Rhin": "rhine",
    "Tamise": "thames",
    "Meuse": "meuse",
    "Escaut": "scheldt",
    "Dordogne": "dordogne",
    "Saône": "saone",
    "Pô": "po",
    "Tage": "tagus",
    "Èbre": "ebro",
    "Danube": "danube",
}

# Terrain classification (cell = 1.44 km; slope = rise / run on that grid).
HILLS_MIN_M = 350.0
HILLS_MIN_SLOPE = 0.05
MOUNTAINS_MIN_M = 1200.0
MOUNTAINS_MIN_SLOPE = 0.14
MOUNTAINS_MIN_ROCK = 0.45
FOREST_MIN_WEIGHT = 0.5
MARSH_MAX_M = 15.0
MARSH_MAX_SLOPE = 0.01
MARSH_TERRAIN = "marsh"

# Crossings.
CROSSING_SNAP_CELLS = 8  # search radius for the named river around a bridge
CROSSING_RADIUS = 1  # river cells reopened around the snapped cell (Chebyshev)
ROAD_CROSSING_MAX_CELLS = 6  # larger road/river overlaps = road along the river
ROAD_SOURCE = "itiner-e"
PASS_WINDOW_CELLS = 30
PASS_SLOPE_PENALTY = 3.0  # cost multiplier of too-steep cells for the pass search

PREVIEW_SIZE = 2048
EIGHT = np.ones((3, 3), dtype=bool)


@dataclass
class Crossing:
    """One ``crossings.json`` entry and what the build did with it."""

    id: str
    name: str
    type: str
    river: str | None
    cell: tuple[int, int]
    applied: bool = False
    cells: list[tuple[int, int]] = field(default_factory=list)


@dataclass
class NavgridLayers:
    """Everything :func:`compute` produces (grid plus diagnostics)."""

    cost: np.ndarray
    water: np.ndarray
    major: np.ndarray
    reopened: np.ndarray
    road: np.ndarray
    crossings: list[Crossing]
    road_crossings: int
    settlement_cells: dict[str, tuple[int, int]]
    ports: set[str]


@dataclass
class NavgridResult:
    """Output of :func:`build`."""

    path: Path
    preview: Path
    bytes: int = 0
    passable_fraction: float = 0.0
    crossings_used: int = 0
    road_crossings: int = 0
    passes: int = 0
    seconds: float = 0.0
    warnings: list[str] = field(default_factory=list)
    off_river: list[str] = field(default_factory=list)


class NavgridError(RuntimeError):
    """A pipeline check failed (isolated settlement)."""


# ----------------------------------------------------------------------------
# Inputs
# ----------------------------------------------------------------------------


def load_rules(path: Path = RULES_PATH) -> dict:
    """``data/movement/rules.json``."""
    return json.loads(path.read_text(encoding="utf-8"))


def load_crossings(path: Path) -> list[dict]:
    """``crossings.json`` entries (empty list without the file)."""
    if not path.exists():
        return []
    return json.loads(path.read_text(encoding="utf-8"))["crossings"]


def to_cell(px: float, py: float, scale: int, size: int) -> tuple[int, int]:
    """Map pixel -> ``(row, col)`` grid cell, clamped."""
    return (
        int(np.clip(py // scale, 0, size - 1)),
        int(np.clip(px // scale, 0, size - 1)),
    )


def rasterize_lines(
    lines: list, size: int, scale: int, all_touched: bool
) -> np.ndarray:
    """Boolean raster of map-pixel ``lines`` (shapely geometries) on the grid."""
    if not lines:
        return np.zeros((size, size), dtype=bool)
    factor = 1.0 / scale
    shapes = [
        (shapely.affinity.scale(line, factor, factor, origin=(0, 0)), 1)
        for line in lines
    ]
    return (
        rasterize(
            shapes,
            out_shape=(size, size),
            fill=0,
            dtype=np.uint8,
            all_touched=all_touched,
        )
        > 0
    )


def four_connected(mask: np.ndarray) -> np.ndarray:
    """Close diagonal-only contacts so the mask blocks 8-neighbour moves.

    Where two cells of ``mask`` touch only by a corner, one of the two
    orthogonal cells between them is added.
    """
    out = mask.copy()
    a, b = mask[:-1, :-1], mask[1:, 1:]
    c, d = mask[:-1, 1:], mask[1:, :-1]
    out[:-1, 1:] |= a & b & ~c & ~d
    out[:-1, :-1] |= c & d & ~a & ~b
    return out


# ----------------------------------------------------------------------------
# Layers
# ----------------------------------------------------------------------------


def terrain_cost(
    height: np.ndarray,
    slope: np.ndarray,
    weights: np.ndarray,
    marsh_province: np.ndarray,
    rules: dict,
) -> np.ndarray:
    """Base cost per cell (float), :data:`IMPASSABLE` above the slope limit.

    Args:
        height: Mean elevation (m) per cell.
        slope: Rise / run per cell.
        weights: Splat weights ``(rows, cols, 4)`` in ``[0, 1]`` (grass, farm, forest, rock).
        marsh_province: Cells of a province whose dominant terrain is ``marsh``.
        rules: ``data/movement/rules.json``.
    """
    costs = rules["terrain_costs"]
    cost = np.full(height.shape, float(costs["plains"]))
    hills = (height >= HILLS_MIN_M) | (slope >= HILLS_MIN_SLOPE)
    forest = weights[..., 2] >= FOREST_MIN_WEIGHT
    marsh = marsh_province & (height <= MARSH_MAX_M) & (slope <= MARSH_MAX_SLOPE)
    mountains = (
        (height >= MOUNTAINS_MIN_M)
        | (slope >= MOUNTAINS_MIN_SLOPE)
        | (weights[..., 3] >= MOUNTAINS_MIN_ROCK)
    )
    for mask, name in (
        (hills, "hills"),
        (forest, "forest"),
        (marsh, "marsh"),
        (mountains, "mountains"),
    ):
        cost = np.where(mask, np.maximum(cost, float(costs[name])), cost)
    cost[slope > rules["slope_impassable_threshold"]] = IMPASSABLE
    return cost


def apply_roads(cost: np.ndarray, road: np.ndarray, factor: float) -> np.ndarray:
    """Roads multiply the cost of passable cells by ``factor``."""
    passable = cost < IMPASSABLE
    return np.where(road & passable, np.maximum(1.0, np.round(cost * factor)), cost)


def road_river_crossings(road: np.ndarray, major: np.ndarray) -> np.ndarray:
    """River cells where an Itiner-e road crosses (small overlaps only)."""
    overlap = road & (major > 0)
    labels, count = ndimage.label(overlap, structure=EIGHT)
    if count == 0:
        return overlap
    sizes = np.bincount(labels.ravel(), minlength=count + 1)
    keep = sizes <= ROAD_CROSSING_MAX_CELLS
    keep[0] = False
    return keep[labels]


def snap_to_river(
    major: np.ndarray, key_index: int, cell: tuple[int, int], radius: int
) -> tuple[int, int] | None:
    """Nearest cell of river ``key_index`` within ``radius`` cells of ``cell``."""
    row, col = cell
    top, left = max(0, row - radius), max(0, col - radius)
    window = major[top : row + radius + 1, left : col + radius + 1] == key_index
    if not window.any():
        return None
    rows, cols = np.nonzero(window)
    distance = np.hypot(rows + top - row, cols + left - col)
    best = int(np.argmin(distance))
    return int(rows[best] + top), int(cols[best] + left)


def open_crossing(
    river: np.ndarray, center: tuple[int, int], radius: int = CROSSING_RADIUS
) -> list[tuple[int, int]]:
    """River cells within ``radius`` (Chebyshev) of ``center``."""
    row, col = center
    top, left = max(0, row - radius), max(0, col - radius)
    window = river[top : row + radius + 1, left : col + radius + 1]
    rows, cols = np.nonzero(window)
    return [(int(r + top), int(c + left)) for r, c in zip(rows, cols, strict=True)]


def carve_pass(
    cost: np.ndarray,
    steep: np.ndarray,
    blocked: np.ndarray,
    route: list[tuple[int, int]],
) -> list[tuple[int, int]]:
    """Least-cost cells along ``route`` (steep cells allowed, ``blocked`` not).

    Returns the cells of the path, closed diagonally, or an empty list when a
    leg cannot be traced.
    """
    rows = [r for r, _ in route]
    cols = [c for _, c in route]
    size = cost.shape[0]
    top = max(0, min(rows) - PASS_WINDOW_CELLS)
    left = max(0, min(cols) - PASS_WINDOW_CELLS)
    bottom = min(size, max(rows) + PASS_WINDOW_CELLS + 1)
    right = min(size, max(cols) + PASS_WINDOW_CELLS + 1)
    window = cost[top:bottom, left:right].astype(np.float64)
    window = np.where(
        steep[top:bottom, left:right], window * PASS_SLOPE_PENALTY, window
    )
    window[blocked[top:bottom, left:right]] = np.inf
    path_mask = np.zeros(window.shape, dtype=bool)
    for (r0, c0), (r1, c1) in zip(route[:-1], route[1:], strict=True):
        start, end = (r0 - top, c0 - left), (r1 - top, c1 - left)
        local = window.copy()
        local[start] = local[end] = 1.0
        graph = MCP_Geometric(local, fully_connected=True)
        costs, _ = graph.find_costs([start], [end])
        if not np.isfinite(costs[end]):
            return []
        for r, c in graph.traceback(end):
            path_mask[r, c] = True
    path_mask = four_connected(path_mask)
    rows_out, cols_out = np.nonzero(path_mask)
    return [
        (int(r + top), int(c + left)) for r, c in zip(rows_out, cols_out, strict=True)
    ]


# ----------------------------------------------------------------------------
# Build
# ----------------------------------------------------------------------------


def _load_rivers(map_dir: Path) -> list[dict]:
    return json.loads((map_dir / "rivers.geojson").read_text(encoding="utf-8"))[
        "features"
    ]


def _load_roads(map_dir: Path) -> list[dict]:
    path = map_dir / settlements.ROADS_FILE
    if not path.exists():
        return []
    return json.loads(path.read_text(encoding="utf-8"))["features"]


def _major_key(name: str | None) -> str | None:
    for key, names in MAJOR_RIVERS.items():
        if name in names:
            return key
    return None


def _marsh_provinces(map_dir: Path, ids: np.ndarray) -> np.ndarray:
    terrains = splat.province_terrains(map_dir / "provinces.geojson")
    marsh = np.zeros(int(ids.max()) + 1, dtype=bool)
    for index, name in terrains.items():
        if name == MARSH_TERRAIN and 0 <= index < len(marsh):
            marsh[index] = True
    return marsh[ids]


def _splat_weights(map_dir: Path, size: int) -> np.ndarray:
    path = map_dir / "splat.png"
    if not path.exists():
        return np.zeros((size, size, 4), dtype=np.float32)
    with Image.open(path) as image:
        array = np.asarray(image.convert("RGBA"), dtype=np.float32) / 255.0
    if array.shape[0] != size:
        step = array.shape[0] // size
        array = array[::step, ::step]
    return array


def compute(
    map_dir: Path = MAP_DIR,
    rules: dict | None = None,
    crossings_path: Path | None = None,
) -> NavgridLayers:
    """Compute the grid and its diagnostic layers from ``data/map``."""
    rules = rules if rules is not None else load_rules()
    crossings_path = crossings_path or map_dir / CROSSINGS_FILE
    size = NAVGRID_SIZE
    height_full = terrain.uint16_to_height(
        terrain.read_png16(map_dir / "heightmap.png")
    )
    scale = height_full.shape[0] // size
    with Image.open(map_dir / "land_mask.png") as image:
        land_full = np.asarray(image.convert("L")) > 127
    height = splat.downsample_mean(height_full, scale)
    # Water as drawn by the game: sea at or below 0 m, or lakes (land mask).
    water = splat.downsample_mean((height_full <= 0.0) | ~land_full, scale) > 0.5
    meters_per_cell = (
        json.loads((map_dir / "map.json").read_text(encoding="utf-8"))["meters_per_px"]
        * scale
    )
    grad_y, grad_x = np.gradient(height, meters_per_cell)
    slope = np.hypot(grad_x, grad_y)
    labels = settlements.load_labels(map_dir)[::scale, ::scale]
    cost = terrain_cost(
        height,
        slope,
        _splat_weights(map_dir, size),
        _marsh_provinces(map_dir, labels),
        rules,
    )
    steep = slope > rules["slope_impassable_threshold"]

    # Rivers.
    major = np.zeros((size, size), dtype=np.int16)
    keys = list(MAJOR_RIVERS)
    minor_lines = []
    major_lines: dict[str, list] = {key: [] for key in keys}
    for feature in _load_rivers(map_dir):
        geometry = shapely.geometry.shape(feature["geometry"])
        key = _major_key(feature["properties"].get("name"))
        if key is None:
            minor_lines.append(geometry)
        else:
            major_lines[key].append(geometry)
    minor = rasterize_lines(minor_lines, size, scale, all_touched=False)
    cost = np.where(
        minor & (cost < IMPASSABLE), cost + rules["minor_river_extra"], cost
    )
    for index, key in enumerate(keys, start=1):
        mask = four_connected(
            rasterize_lines(major_lines[key], size, scale, all_touched=True)
        )
        major[mask & (major == 0)] = index
    river = four_connected(major > 0)
    # Cells added by the global closing (two rivers touching by a corner).
    major[river & (major == 0)] = 1

    # Roads.
    roads = _load_roads(map_dir)
    road = rasterize_lines(
        [shapely.geometry.shape(f["geometry"]) for f in roads],
        size,
        scale,
        all_touched=False,
    )
    itinere = rasterize_lines(
        [
            shapely.geometry.shape(f["geometry"])
            for f in roads
            if f["properties"].get("source") == ROAD_SOURCE
        ],
        size,
        scale,
        all_touched=False,
    )
    cost = apply_roads(cost, road, rules["road_cost_factor"])

    # Reopened river cells keep their terrain (and road) cost.
    reopened = road_river_crossings(itinere, major)
    road_crossings = int(ndimage.label(reopened, structure=EIGHT)[1])
    crossings = []
    grid_scale = scale
    map_grid = settlements.provinces_step.load_grid(map_dir)
    for entry in load_crossings(crossings_path):
        px, py = _lonlat_to_pixel(map_grid, *entry["lonlat"])
        crossing = Crossing(
            id=entry["id"],
            name=entry["name"],
            type=entry["type"],
            river=entry.get("river"),
            cell=to_cell(px, py, grid_scale, size),
        )
        crossings.append(crossing)
        if crossing.type == "pass":
            continue
        key = RIVER_FR.get(crossing.river or "")
        if key is None:
            continue
        snapped = snap_to_river(
            major, keys.index(key) + 1, crossing.cell, CROSSING_SNAP_CELLS
        )
        if snapped is None:
            continue
        crossing.cells = open_crossing(river, snapped)
        crossing.applied = True
        for cell in crossing.cells:
            reopened[cell] = True
    cost = np.where(river & ~reopened, IMPASSABLE, cost)

    # Passes: forced passable along their route (mountain cost at most).
    blocked = water | (river & ~reopened)
    mountains = float(rules["terrain_costs"]["mountains"])
    for crossing, entry in zip(crossings, load_crossings(crossings_path), strict=True):
        if crossing.type != "pass":
            continue
        route = [
            to_cell(*_lonlat_to_pixel(map_grid, lon, lat), grid_scale, size)
            for lon, lat in entry["route"]
        ]
        crossing.cells = carve_pass(
            np.where(steep, mountains, np.minimum(cost, MAX_COST)),
            steep,
            blocked,
            route,
        )
        crossing.applied = bool(crossing.cells)
        for cell in crossing.cells:
            if cost[cell] >= IMPASSABLE and not blocked[cell]:
                cost[cell] = mountains

    cost[water] = IMPASSABLE
    positions = json.loads(
        (map_dir / settlements.POSITIONS_FILE).read_text(encoding="utf-8")
    )
    settlement_cells = {
        sid: to_cell(x, y, grid_scale, size) for sid, (x, y) in positions.items()
    }
    plains = float(rules["terrain_costs"]["plains"])
    for cell in settlement_cells.values():
        cost[cell] = plains
    grid = np.clip(np.round(cost), 1, IMPASSABLE).astype(np.uint8)
    return NavgridLayers(
        cost=grid,
        water=water,
        major=major,
        reopened=reopened,
        road=road,
        crossings=crossings,
        road_crossings=road_crossings,
        settlement_cells=settlement_cells,
        ports=_ports(),
    )


def _ports() -> set[str]:
    provinces = settlements.load_provinces()
    return {s.id for s in settlements.load_settlements(provinces) if s.port}


def _lonlat_to_pixel(grid: MapGrid, lon: float, lat: float) -> tuple[float, float]:
    px, py = grid.lonlat_to_pixel(lon, lat)
    return float(px), float(py)


# ----------------------------------------------------------------------------
# Checks
# ----------------------------------------------------------------------------


def land_masses(layers: NavgridLayers) -> np.ndarray:
    """8-connected land components (settlement cells count as land)."""
    land = ~layers.water
    for cell in layers.settlement_cells.values():
        land[cell] = True
    labels, _ = ndimage.label(land, structure=EIGHT)
    return labels


def check(layers: NavgridLayers) -> tuple[list[str], list[str]]:
    """Connectivity errors and warnings (islands without port)."""
    errors: list[str] = []
    warnings: list[str] = []
    masses = land_masses(layers)
    passable, _ = ndimage.label(layers.cost < IMPASSABLE, structure=EIGHT)
    groups: dict[int, list[str]] = {}
    for sid, cell in layers.settlement_cells.items():
        groups.setdefault(int(masses[cell]), []).append(sid)
    for mass, members in sorted(groups.items(), key=lambda item: -len(item[1])):
        components: dict[int, list[str]] = {}
        for sid in members:
            components.setdefault(
                int(passable[layers.settlement_cells[sid]]), []
            ).append(sid)
        if len(components) > 1:
            parts = sorted(components.values(), key=len, reverse=True)
            for part in parts[1:]:
                errors.append(
                    f"masse terrestre {mass} : {', '.join(sorted(part))} "
                    f"isolée(s) de {parts[0][0]} ({len(parts[0])} colonies)"
                )
        if not any(sid in layers.ports for sid in members):
            warnings.append(
                f"île sans port (masse {mass}) : {', '.join(sorted(members))}"
            )
    return errors, warnings


# ----------------------------------------------------------------------------
# Outputs
# ----------------------------------------------------------------------------

PREVIEW_COLOURS = {
    "water": (70, 110, 160),
    "impassable": (40, 36, 34),
    "river": (20, 60, 230),
    "reopened": (255, 40, 40),
    "road": (230, 150, 40),
}


def render_preview(layers: NavgridLayers, rules: dict, path: Path) -> None:
    """Colour map of the costs: terrain ramp, roads, rivers, crossings, settlements."""
    cost = layers.cost.astype(np.float32)
    costs = rules["terrain_costs"]
    low, high = float(costs["plains"]) * 0.5, float(costs["mountains"]) + 20.0
    t = np.clip((cost - low) / (high - low), 0.0, 1.0)[..., None]
    ramp = (1 - t) * np.array([190, 215, 140]) + t * np.array([120, 70, 40])
    rgb = ramp.astype(np.uint8)
    impassable = layers.cost >= IMPASSABLE
    river = layers.major > 0
    rgb[layers.road & ~impassable] = PREVIEW_COLOURS["road"]
    rgb[impassable] = PREVIEW_COLOURS["impassable"]
    rgb[layers.water] = PREVIEW_COLOURS["water"]
    rgb[river & impassable & ~layers.water] = PREVIEW_COLOURS["river"]
    rgb[layers.reopened & river] = PREVIEW_COLOURS["reopened"]
    image = Image.fromarray(rgb, mode="RGB")
    draw = ImageDraw.Draw(image)
    for crossing in layers.crossings:
        if crossing.type == "pass":
            for r, c in crossing.cells:
                image.putpixel((c, r), (255, 230, 0))
        elif crossing.applied:
            r, c = crossing.cell
            draw.ellipse((c - 3, r - 3, c + 3, r + 3), outline=(255, 40, 40))
    for r, c in layers.settlement_cells.values():
        draw.rectangle((c - 1, r - 1, c + 1, r + 1), fill=(255, 255, 255))
    if image.size[0] != PREVIEW_SIZE:
        image = image.resize((PREVIEW_SIZE, PREVIEW_SIZE), Image.Resampling.NEAREST)
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)


def update_map_json(map_dir: Path, scale: int) -> None:
    """Add (or refresh) ``navgrid`` in ``map.json``, keeping every other key."""
    path = map_dir / "map.json"
    metadata = json.loads(path.read_text(encoding="utf-8"))
    metadata["navgrid"] = {
        "size_px": NAVGRID_SIZE,
        "file": NAVGRID_FILE,
        "scale": scale,
    }
    path.write_text(json.dumps(metadata, indent=2) + "\n", encoding="utf-8")


def build(
    map_dir: Path = MAP_DIR,
    preview_path: Path = PREVIEW_PATH,
    rules_path: Path = RULES_PATH,
    strict: bool = True,
) -> NavgridResult:
    """Write ``navgrid.png``, its preview and ``map.json.navgrid``.

    Raises:
        NavgridError: ``strict`` and a settlement is cut off from its land mass.
    """
    started = time.perf_counter()
    rules = load_rules(rules_path)
    layers = compute(map_dir, rules)
    errors, warnings = check(layers)
    off_river = []
    for crossing in layers.crossings:
        if crossing.applied:
            continue
        if crossing.type == "pass":
            warnings.append(f"col non tracé : {crossing.id}")
        elif crossing.river in RIVER_FR:
            # The river is not traced there in rivers.geojson (e.g. lower Saône).
            off_river.append(crossing.id)
    render_preview(layers, rules, preview_path)
    if errors and strict:
        raise NavgridError(
            "colonies isolées par la grille de navigation :\n" + "\n".join(errors)
        )
    warnings = errors + warnings
    path = map_dir / NAVGRID_FILE
    terrain.write_png8(layers.cost, path)
    scale = settlements.provinces_step.load_grid(map_dir).size_px // NAVGRID_SIZE
    update_map_json(map_dir, scale)
    land = ~layers.water
    return NavgridResult(
        path=path,
        preview=preview_path,
        bytes=path.stat().st_size,
        passable_fraction=float((layers.cost[land] < IMPASSABLE).mean()),
        crossings_used=sum(
            1 for c in layers.crossings if c.applied and c.type != "pass"
        ),
        road_crossings=layers.road_crossings,
        passes=sum(1 for c in layers.crossings if c.applied and c.type == "pass"),
        seconds=time.perf_counter() - started,
        warnings=warnings,
        off_river=off_river,
    )
