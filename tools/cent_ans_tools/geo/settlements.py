"""Settlement positions and movement graph (lot C3, spec 2026-09-24 § 4.1 and § 4.4).

Inputs: ``data/settlements/*.json`` (provinces without a file, or without a
``city``, get the same fallback city as the Rust loader), ``data/provinces``,
``data/map/province_ids.png``, ``data/map/provinces.geojson`` and, when
present, ``data/map/roads.geojson``.

Outputs:
    * ``data/map/settlements_px.json``: ``{settlement id: [x, y]}`` in map
      pixels (game position, snapped into its province when the data point
      falls outside it).
    * ``data/map/settlement_graph.json``: ``{"edges": [{from, to, cost, road,
      sea}]}``, undirected (one entry per pair, ``from < to``), exactly the
      format read by ``core/crates/data-model/src/settlement_load.rs``.
    * ``data/map/settlement_edge_paths.json``: road polyline of every ``road``
      edge, for display (see :mod:`cent_ans_tools.geo.edge_paths`, lot C7b).
    * ``docs/img/settlements-preview.png``.

Graph (§ 4.4):
    * inside a province: Delaunay triangulation of its settlements, edges longer
      than :data:`LONG_EDGE_FACTOR` x the province median removed unless they
      belong to the minimum spanning tree (the province stays connected);
    * between land neighbours: the 1 to :data:`MAX_BORDER_PAIRS` closest pairs
      of settlements, pairwise disjoint and within :data:`BORDER_PAIR_SLACK`
      x the closest distance;
    * between ``sea_neighbors``: the closest pair of ``port`` settlements.

Cost: length in km x terrain cost (the ``terrain_cost`` of
``core/crates/sim-campaign/src/movement.rs``: 2 for mountains and marsh, 1
otherwise; mean of both provinces across a border), halved on a road. Sea
edges cost :data:`SEA_FIXED_COST` + km. The cost is unitless: the core
rescales it into movement points.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import distance_transform_edt
from scipy.sparse import coo_matrix
from scipy.sparse.csgraph import connected_components, minimum_spanning_tree
from scipy.spatial import Delaunay, QhullError

from cent_ans_tools.geo import edge_paths as edge_paths_step
from cent_ans_tools.geo import provinces as provinces_step
from cent_ans_tools.geo.project import MapGrid

REPO_DIR = Path(__file__).resolve().parents[3]
MAP_DIR = REPO_DIR / "data" / "map"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"
SETTLEMENTS_DIR = REPO_DIR / "data" / "settlements"
PREVIEW_PATH = REPO_DIR / "docs" / "img" / "settlements-preview.png"
RULES_FILE = "rules.json"
GRAPH_FILE = "settlement_graph.json"
POSITIONS_FILE = "settlements_px.json"
ROADS_FILE = "roads.geojson"

SETTLEMENT_PREFIX = "set_"
PROVINCE_PREFIX = "prov_"
LONG_EDGE_FACTOR = 2.5
MAX_BORDER_PAIRS = 3
BORDER_PAIR_SLACK = 1.5
# Mirrors `terrain_cost` in core/crates/sim-campaign/src/movement.rs.
TERRAIN_COST = {"mountains": 2.0, "marsh": 2.0}
DEFAULT_TERRAIN_COST = 1.0
ROAD_DIVISOR = 2.0
# Embarking and landing an army: about one province crossing on land.
SEA_FIXED_COST = 100.0
STRAIT_WATER_FRACTION = 0.5
ROAD_MAX_MEAN_DISTANCE_KM = 3.0
ROAD_MAX_MEAN_DISTANCE_RATIO = 0.04
# Snapped settlements land this far inside their province, and apart.
SNAP_INSET_PX = 3.0
SNAP_MIN_SEPARATION_PX = 4.0
PREVIEW_SIZE = 2048

KIND_COLOURS = {
    "city": (200, 30, 30),
    "town": (240, 150, 20),
    "castle": (60, 60, 60),
    "abbey": (130, 60, 170),
    "village": (40, 140, 60),
}

# Same folding table as `fold_char` in core/crates/data-model/src/entities/settlement.rs.
_FOLD = {
    **dict.fromkeys("àáâãäåÀÁÂÃÄÅ", "a"),
    **dict.fromkeys("çÇ", "c"),
    **dict.fromkeys("èéêëÈÉÊË", "e"),
    **dict.fromkeys("ìíîïÌÍÎÏ", "i"),
    **dict.fromkeys("ñÑ", "n"),
    **dict.fromkeys("òóôõöøÒÓÔÕÖØ", "o"),
    **dict.fromkeys("ùúûüÙÚÛÜ", "u"),
    **dict.fromkeys("ýÿÝ", "y"),
    **dict.fromkeys("æÆ", "ae"),
    **dict.fromkeys("œŒ", "oe"),
    "ß": "ss",
    **dict.fromkeys("ðÐ", "d"),
    **dict.fromkeys("þÞ", "th"),
}


def slugify(text: str) -> str:
    """ASCII slug identical to Rust ``slugify``: accents folded, ``[a-z0-9]`` runs joined by ``_``."""
    folded = []
    for char in text:
        if char in _FOLD:
            folded.append(_FOLD[char])
        elif char.isascii() and char.isalnum():
            folded.append(char.lower())
        else:
            folded.append("_")
    return "_".join(part for part in "".join(folded).split("_") if part)


@dataclass
class Settlement:
    """A settlement as seen by the geo pipeline."""

    id: str
    province: str
    kind: str
    name: str
    lonlat: tuple[float, float]
    port: bool
    fallback: bool = False
    px: tuple[float, float] = (0.0, 0.0)
    snapped: bool = False


@dataclass
class Edge:
    """One undirected edge of the movement graph (``a < b`` by id)."""

    a: str
    b: str
    km: float
    terrain_cost: float
    sea: bool = False
    road: bool = False

    @property
    def cost(self) -> float:
        """Unitless movement cost (see module docstring)."""
        if self.sea:
            return SEA_FIXED_COST + self.km
        cost = self.km * self.terrain_cost
        return cost / ROAD_DIVISOR if self.road else cost

    def to_json(self) -> dict:
        """Entry of ``settlement_graph.json``."""
        return {
            "from": self.a,
            "to": self.b,
            "cost": round(self.cost, 2),
            "road": self.road,
            "sea": self.sea,
        }


@dataclass
class SettlementResult:
    """Outputs of :func:`build` and diagnostics for the report."""

    graph: Path
    positions: Path
    preview: Path
    seconds: float
    edge_paths: Path | None = None
    traced_edges: int = 0
    settlements: int = 0
    fallback_cities: list[str] = field(default_factory=list)
    snapped: list[str] = field(default_factory=list)
    land_edges: int = 0
    road_edges: int = 0
    sea_edges: int = 0
    components: int = 0
    warnings: list[str] = field(default_factory=list)


# ----------------------------------------------------------------------------
# Inputs
# ----------------------------------------------------------------------------


def load_provinces(provinces_dir: Path = PROVINCES_DIR) -> dict[str, dict]:
    """Raw ``data/provinces/*.json`` by id, sorted."""
    result = {}
    for path in sorted(provinces_dir.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        result[data["id"]] = data
    return result


def fallback_city(province: dict, settlement_id: str) -> Settlement:
    """Same city as ``Settlement::fallback_city`` in Rust."""
    city = province["capital_city"]
    if city.get("lon") is not None and city.get("lat") is not None:
        lonlat = (float(city["lon"]), float(city["lat"]))
    elif province.get("geo"):
        lonlat = tuple(province["geo"]["capital_lonlat"])
    else:
        lonlat = (0.0, 0.0)
    return Settlement(
        id=settlement_id,
        province=province["id"],
        kind="city",
        name=city["name"]["display"],
        lonlat=lonlat,
        port=bool(province.get("coastal")) and bool(province.get("ports")),
        fallback=True,
    )


def load_settlements(
    provinces: dict[str, dict],
    settlements_dir: Path = SETTLEMENTS_DIR,
    warnings: list[str] | None = None,
) -> list[Settlement]:
    """Settlement files plus Rust-identical fallback cities, sorted by province then file order.

    Mirrors ``GameData::load_settlements``: entries of unknown provinces and
    duplicate ids are skipped; a province without a ``city`` receives one
    generated from ``capital_city`` with id ``set_<slug>`` (or
    ``set_<slug>_<province suffix>`` when a file already uses it, or
    ``set_<province suffix>`` when the name has no usable character).
    """
    warnings = warnings if warnings is not None else []
    by_id: dict[str, Settlement] = {}
    paths = (
        sorted(p for p in settlements_dir.glob("*.json") if p.name != RULES_FILE)
        if settlements_dir.is_dir()
        else []
    )
    for path in paths:
        for entry in json.loads(path.read_text(encoding="utf-8")):
            if entry["province"] not in provinces:
                warnings.append(
                    f"{entry['id']} : province inconnue {entry['province']}"
                )
                continue
            if entry["id"] in by_id:
                warnings.append(f"{entry['id']} : identifiant en double, ignoré")
                continue
            by_id[entry["id"]] = Settlement(
                id=entry["id"],
                province=entry["province"],
                kind=entry["kind"],
                name=entry["name"]["display"],
                lonlat=(float(entry["lonlat"][0]), float(entry["lonlat"][1])),
                port=bool(entry.get("port", False)),
            )
    has_city = {s.province for s in by_id.values() if s.kind == "city"}
    generated = []
    for province_id, province in provinces.items():
        if province_id in has_city:
            continue
        suffix = province_id.removeprefix(PROVINCE_PREFIX)
        slug = slugify(province["capital_city"]["name"]["display"])
        if not slug:
            settlement_id = f"{SETTLEMENT_PREFIX}{suffix}"
        elif f"{SETTLEMENT_PREFIX}{slug}" in by_id:
            settlement_id = f"{SETTLEMENT_PREFIX}{slug}_{suffix}"
        else:
            settlement_id = f"{SETTLEMENT_PREFIX}{slug}"
        generated.append(fallback_city(province, settlement_id))
    for city in generated:
        by_id[city.id] = city
    order = {province_id: index for index, province_id in enumerate(provinces)}
    return sorted(
        by_id.values(), key=lambda s: (order[s.province], s.kind != "city", s.id)
    )


def load_province_geometry(map_dir: Path = MAP_DIR) -> dict[str, dict]:
    """``provinces.geojson`` properties by id (``index``, ``neighbors``, ``sea_neighbors``)."""
    collection = json.loads((map_dir / "provinces.geojson").read_text(encoding="utf-8"))
    return {
        feature["properties"]["id"]: feature["properties"]
        for feature in collection["features"]
    }


def load_labels(map_dir: Path = MAP_DIR) -> np.ndarray:
    """``province_ids.png`` decoded into province indices (0 = none)."""
    with Image.open(map_dir / "province_ids.png") as image:
        return provinces_step.decode_ids(np.asarray(image.convert("RGB")))


# ----------------------------------------------------------------------------
# Projection
# ----------------------------------------------------------------------------


def snap_into_province(
    labels: np.ndarray,
    index: int,
    x: float,
    y: float,
    occupied: list[tuple[float, float]],
) -> tuple[float, float]:
    """Nearest pixel centre of province ``index`` at least :data:`SNAP_INSET_PX` inside it.

    Pixels closer than :data:`SNAP_MIN_SEPARATION_PX` to an ``occupied``
    position are skipped so that several snapped settlements do not stack up.
    The inset shrinks when the province is too thin.
    """
    rows, cols = np.nonzero(labels == index)
    top, left = rows.min(), cols.min()
    mask = np.zeros((rows.max() - top + 3, cols.max() - left + 3), dtype=bool)
    mask[rows - top + 1, cols - left + 1] = True
    inside = distance_transform_edt(mask)[rows - top + 1, cols - left + 1]
    inset = min(SNAP_INSET_PX, float(inside.max()))
    keep = inside >= inset
    rows, cols = rows[keep], cols[keep]
    centres = np.column_stack([cols + 0.5, rows + 0.5]).astype(np.float64)
    if occupied:
        others = np.asarray(occupied, dtype=np.float64)
        gap = np.min(
            np.hypot(
                centres[:, None, 0] - others[None, :, 0],
                centres[:, None, 1] - others[None, :, 1],
            ),
            axis=1,
        )
        free = gap >= SNAP_MIN_SEPARATION_PX
        if free.any():
            centres = centres[free]
    nearest = int(np.argmin((centres[:, 0] - x) ** 2 + (centres[:, 1] - y) ** 2))
    return float(centres[nearest, 0]), float(centres[nearest, 1])


def place_settlements(
    settlements: list[Settlement],
    grid: MapGrid,
    labels: np.ndarray,
    index_of: dict[str, int],
) -> list[str]:
    """Set ``px`` of every settlement, snapping it into its province if needed.

    Returns:
        Ids of the snapped settlements. Only the game position moves; the
        data ``lonlat`` is untouched.
    """
    lon = np.array([s.lonlat[0] for s in settlements])
    lat = np.array([s.lonlat[1] for s in settlements])
    xs, ys = grid.lonlat_to_pixel(lon, lat)
    height, width = labels.shape
    outside = []
    for settlement, x, y in zip(settlements, xs, ys, strict=True):
        # Truncated to 0.1 px (the output precision) so the pixel never changes.
        settlement.px = (float(np.floor(x * 10) / 10), float(np.floor(y * 10) / 10))
        row, col = int(y), int(x)
        inside = 0 <= row < height and 0 <= col < width
        settlement.snapped = not (
            inside and labels[row, col] == index_of[settlement.province]
        )
        if settlement.snapped:
            outside.append(settlement)
    occupied = [s.px for s in settlements if not s.snapped]
    for settlement in outside:
        settlement.px = snap_into_province(
            labels, index_of[settlement.province], *settlement.px, occupied
        )
        occupied.append(settlement.px)
    return [s.id for s in outside]


# ----------------------------------------------------------------------------
# Graph
# ----------------------------------------------------------------------------


def province_edges(points: np.ndarray) -> list[tuple[int, int]]:
    """Delaunay edges of ``points`` without the long ones, kept connected.

    Args:
        points: ``(n, 2)`` positions of one province's settlements.

    Returns:
        Index pairs ``(i, j)`` with ``i < j``.
    """
    count = len(points)
    if count < 2:
        return []
    if count == 2:
        return [(0, 1)]
    candidates: set[tuple[int, int]] = set()
    try:
        triangles = Delaunay(points, qhull_options="QJ").simplices
        for triangle in triangles:
            for k in range(3):
                i, j = int(triangle[k]), int(triangle[(k + 1) % 3])
                candidates.add((min(i, j), max(i, j)))
    except QhullError:
        candidates = {(i, j) for i in range(count) for j in range(i + 1, count)}
    pairs = sorted(candidates)
    lengths = np.array([np.hypot(*(points[i] - points[j])) for i, j in pairs])
    # Coincident points still need an edge: keep a tiny positive weight for the MST.
    weights = np.maximum(lengths, 1e-6)
    matrix = coo_matrix(
        (weights, ([i for i, _ in pairs], [j for _, j in pairs])), shape=(count, count)
    )
    tree = minimum_spanning_tree(matrix).tocoo()
    tree_pairs = {
        (min(int(i), int(j)), max(int(i), int(j)))
        for i, j in zip(tree.row, tree.col, strict=True)
    }
    limit = LONG_EDGE_FACTOR * float(np.median(lengths))
    return [
        pair
        for pair, length in zip(pairs, lengths, strict=True)
        if length <= limit or pair in tree_pairs
    ]


def closest_pairs(
    points_a: np.ndarray,
    points_b: np.ndarray,
    max_pairs: int = MAX_BORDER_PAIRS,
    slack: float = BORDER_PAIR_SLACK,
) -> list[tuple[int, int]]:
    """1 to ``max_pairs`` closest ``(i in a, j in b)`` pairs, sharing no settlement.

    Pairs after the first are kept only within ``slack`` x the closest distance.
    """
    if len(points_a) == 0 or len(points_b) == 0:
        return []
    distances = np.hypot(
        points_a[:, None, 0] - points_b[None, :, 0],
        points_a[:, None, 1] - points_b[None, :, 1],
    )
    order = np.argsort(distances, axis=None, kind="stable")
    best = float(distances.flat[order[0]])
    chosen: list[tuple[int, int]] = []
    used_a: set[int] = set()
    used_b: set[int] = set()
    for flat in order:
        i, j = np.unravel_index(int(flat), distances.shape)
        if distances[i, j] > max(best * slack, best + 1e-9):
            break
        if i in used_a or j in used_b:
            continue
        chosen.append((int(i), int(j)))
        used_a.add(int(i))
        used_b.add(int(j))
        if len(chosen) == max_pairs:
            break
    return chosen


def terrain_cost(terrain: str) -> float:
    """Per-km multiplier of a province terrain (movement.rs ``terrain_cost``)."""
    return TERRAIN_COST.get(terrain, DEFAULT_TERRAIN_COST)


def build_edges(
    settlements: list[Settlement],
    provinces: dict[str, dict],
    geometry: dict[str, dict],
    km_per_px: float,
    warnings: list[str] | None = None,
    labels: np.ndarray | None = None,
) -> list[Edge]:
    """Every edge of the graph (``road`` not set yet), sorted by ``(a, b)``.

    With ``labels``, a border edge whose straight line runs mostly over water
    (more than :data:`STRAIT_WATER_FRACTION`, e.g. Messina, Øresund) becomes a
    ``sea`` edge when both ends are ports; otherwise it stays on land and a
    warning is reported.
    """
    warnings = warnings if warnings is not None else []
    by_province: dict[str, list[Settlement]] = {}
    for settlement in settlements:
        by_province.setdefault(settlement.province, []).append(settlement)
    positions = {
        province: np.array([s.px for s in members], dtype=np.float64)
        for province, members in by_province.items()
    }
    edges: dict[tuple[str, str], Edge] = {}

    def add(first: Settlement, second: Settlement, cost: float, sea: bool) -> None:
        a, b = sorted((first.id, second.id))
        if a == b or (a, b) in edges:
            return
        km = float(np.hypot(first.px[0] - second.px[0], first.px[1] - second.px[1]))
        edges[(a, b)] = Edge(a, b, km * km_per_px, cost, sea=sea)

    for province, members in by_province.items():
        cost = terrain_cost(provinces[province].get("terrain", ""))
        for i, j in province_edges(positions[province]):
            add(members[i], members[j], cost, False)

    for province, members in by_province.items():
        props = geometry.get(province, {})
        for other in props.get("neighbors", []):
            if other <= province or other not in by_province:
                continue
            cost = (
                terrain_cost(provinces[province].get("terrain", ""))
                + terrain_cost(provinces[other].get("terrain", ""))
            ) / 2.0
            for i, j in closest_pairs(positions[province], positions[other]):
                first, second = members[i], by_province[other][j]
                water = water_fraction(labels, first.px, second.px)
                if water <= STRAIT_WATER_FRACTION:
                    add(first, second, cost, False)
                elif first.port and second.port:
                    add(first, second, 1.0, True)
                else:
                    warnings.append(
                        f"arête {first.id} – {second.id} à {water:.0%} sur l'eau "
                        "gardée terrestre (pas de port aux deux bouts)"
                    )
                    add(first, second, cost, False)
        for other in props.get("sea_neighbors", []):
            if other <= province or other not in by_province:
                continue
            ports_a = [s for s in members if s.port]
            ports_b = [s for s in by_province[other] if s.port]
            if not ports_a or not ports_b:
                missing = province if not ports_a else other
                warnings.append(
                    f"liaison maritime {province} – {other} impossible : aucun port dans {missing}"
                )
                continue
            pairs = closest_pairs(
                np.array([s.px for s in ports_a]),
                np.array([s.px for s in ports_b]),
                max_pairs=1,
            )
            i, j = pairs[0]
            add(ports_a[i], ports_b[j], 1.0, True)
    return [edges[key] for key in sorted(edges)]


def water_fraction(
    labels: np.ndarray | None, start: tuple[float, float], end: tuple[float, float]
) -> float:
    """Share of a straight segment outside every province (0 without ``labels``)."""
    if labels is None:
        return 0.0
    samples = segment_samples(start, end)
    cols = np.clip(samples[:, 0].astype(int), 0, labels.shape[1] - 1)
    rows = np.clip(samples[:, 1].astype(int), 0, labels.shape[0] - 1)
    return float((labels[rows, cols] == 0).mean())


def segment_samples(start: tuple[float, float], end: tuple[float, float]) -> np.ndarray:
    """Points every ~0.5 px along a segment (both ends included), ``(n, 2)``."""
    length = float(np.hypot(end[0] - start[0], end[1] - start[1]))
    count = max(2, int(np.ceil(length * 2)) + 1)
    t = np.linspace(0.0, 1.0, count)[:, None]
    return np.asarray(start) * (1 - t) + np.asarray(end) * t


def flag_roads(
    edges: list[Edge],
    positions: dict[str, tuple[float, float]],
    road_distance_px: np.ndarray,
    max_mean_px: float,
) -> None:
    """Set ``road`` on land edges that a road follows.

    The straight edge must stay, on average, within ``max_mean_px`` of a road,
    or within :data:`ROAD_MAX_MEAN_DISTANCE_RATIO` of its length for long edges
    (a road winding 8 km off a 200 km straight line still follows it).
    """
    height, width = road_distance_px.shape
    for edge in edges:
        if edge.sea:
            continue
        start, end = positions[edge.a], positions[edge.b]
        samples = segment_samples(start, end)
        cols = np.clip(samples[:, 0].astype(int), 0, width - 1)
        rows = np.clip(samples[:, 1].astype(int), 0, height - 1)
        length = float(np.hypot(end[0] - start[0], end[1] - start[1]))
        limit = max(max_mean_px, ROAD_MAX_MEAN_DISTANCE_RATIO * length)
        edge.road = float(road_distance_px[rows, cols].mean()) < limit


def road_distance_raster(
    roads_path: Path, size_px: int | tuple[int, int]
) -> np.ndarray | None:
    """Euclidean distance (px) to the nearest road pixel, or ``None`` without roads.

    ``size_px`` is the map shape ``(rows, cols)`` (an int for a square map).
    """
    if not roads_path.exists():
        return None
    from rasterio.features import rasterize
    from shapely.geometry import shape

    collection = json.loads(roads_path.read_text(encoding="utf-8"))
    shapes = [(shape(f["geometry"]), 1) for f in collection["features"]]
    if not shapes:
        return None
    raster = rasterize(
        shapes,
        out_shape=size_px if isinstance(size_px, tuple) else (size_px, size_px),
        fill=0,
        dtype=np.uint8,
        all_touched=True,
    )
    return distance_transform_edt(raster == 0)


def component_count(settlements: list[Settlement], edges: list[Edge]) -> int:
    """Number of connected components of the graph."""
    index = {s.id: i for i, s in enumerate(settlements)}
    rows = [index[e.a] for e in edges]
    cols = [index[e.b] for e in edges]
    matrix = coo_matrix(
        (np.ones(len(edges)), (rows, cols)), shape=(len(settlements),) * 2
    )
    count, _ = connected_components(matrix, directed=False)
    return int(count)


# ----------------------------------------------------------------------------
# Preview
# ----------------------------------------------------------------------------


def render_preview(
    labels: np.ndarray,
    settlements: list[Settlement],
    edges: list[Edge],
    path: Path,
    size: int = PREVIEW_SIZE,
) -> None:
    """Provinces in pastel tones, edges (roads in brown, sea dashed blue), settlements by kind.

    ``size`` is the preview width; the height follows the map aspect.
    """
    factor = labels.shape[1] / size
    height = int(round(labels.shape[0] / factor))
    step = max(1, int(round(factor)))
    small = labels[::step, ::step]
    rng = np.random.default_rng(1337)
    palette = 0.72 + 0.22 * rng.random((int(labels.max()) + 1, 3))
    palette[0] = (0.62, 0.74, 0.86)
    rgb = palette[small]
    border = np.zeros(small.shape, dtype=bool)
    border[:, :-1] |= small[:, :-1] != small[:, 1:]
    border[:-1, :] |= small[:-1, :] != small[1:, :]
    rgb[border] = 0.35
    image = Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8), mode="RGB")
    if image.size != (size, height):
        image = image.resize((size, height), Image.Resampling.NEAREST)
    draw = ImageDraw.Draw(image)
    position = {s.id: (s.px[0] / factor, s.px[1] / factor) for s in settlements}
    for edge in sorted(edges, key=lambda e: (e.road, e.sea)):
        start, end = position[edge.a], position[edge.b]
        if edge.sea:
            samples = segment_samples(start, end)
            for k in range(0, len(samples) - 1, 8):
                chunk = samples[k : k + 5]
                draw.line([tuple(p) for p in chunk], fill=(30, 80, 200), width=1)
        elif edge.road:
            draw.line([start, end], fill=(130, 75, 20), width=2)
        else:
            draw.line([start, end], fill=(110, 110, 110), width=1)
    for settlement in settlements:
        x, y = position[settlement.id]
        radius = 3.5 if settlement.kind == "city" else 2.2
        draw.ellipse(
            (x - radius, y - radius, x + radius, y + radius),
            fill=KIND_COLOURS.get(settlement.kind, (0, 0, 0)),
            outline=(255, 255, 255) if settlement.snapped else (0, 0, 0),
        )
    legend = [
        ("cité", "city"),
        ("ville", "town"),
        ("château", "castle"),
        ("abbaye", "abbey"),
        ("village", "village"),
    ]
    for k, (label, kind) in enumerate(legend):
        y = 12 + 16 * k
        draw.ellipse((10, y, 20, y + 10), fill=KIND_COLOURS[kind], outline=(0, 0, 0))
        draw.text((26, y - 1), label, fill=(0, 0, 0))
    y = 12 + 16 * len(legend)
    draw.line([(8, y + 5), (22, y + 5)], fill=(130, 75, 20), width=2)
    draw.text((26, y - 1), "route", fill=(0, 0, 0))
    draw.line([(8, y + 21), (22, y + 21)], fill=(30, 80, 200), width=1)
    draw.text((26, y + 15), "mer", fill=(0, 0, 0))
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)


# ----------------------------------------------------------------------------
# Orchestration
# ----------------------------------------------------------------------------


def prepare(
    map_dir: Path = MAP_DIR,
    provinces_dir: Path = PROVINCES_DIR,
    settlements_dir: Path = SETTLEMENTS_DIR,
    warnings: list[str] | None = None,
) -> tuple[list[Settlement], list[Edge], MapGrid, np.ndarray, dict[str, dict]]:
    """Load and place settlements and build the graph topology (no road flags)."""
    warnings = warnings if warnings is not None else []
    grid = provinces_step.load_grid(map_dir)
    provinces = load_provinces(provinces_dir)
    geometry = load_province_geometry(map_dir)
    labels = load_labels(map_dir)
    settlements = load_settlements(provinces, settlements_dir, warnings)
    index_of = {pid: props["index"] for pid, props in geometry.items()}
    place_settlements(settlements, grid, labels, index_of)
    edges = build_edges(
        settlements,
        provinces,
        geometry,
        grid.meters_per_px / 1000.0,
        warnings,
        labels,
    )
    return settlements, edges, grid, labels, provinces


def build(
    map_dir: Path = MAP_DIR,
    provinces_dir: Path = PROVINCES_DIR,
    settlements_dir: Path = SETTLEMENTS_DIR,
    preview_path: Path = PREVIEW_PATH,
) -> SettlementResult:
    """Write ``settlements_px.json``, ``settlement_graph.json`` and the preview."""
    started = time.perf_counter()
    warnings: list[str] = []
    settlements, edges, grid, labels, _ = prepare(
        map_dir, provinces_dir, settlements_dir, warnings
    )
    positions = {s.id: s.px for s in settlements}
    distance = road_distance_raster(map_dir / ROADS_FILE, grid.shape)
    if distance is not None:
        max_px = ROAD_MAX_MEAN_DISTANCE_KM * 1000.0 / grid.meters_per_px
        flag_roads(edges, positions, distance, max_px)
    else:
        warnings.append(f"{ROADS_FILE} absent : aucune arête marquée route")

    positions_path = map_dir / POSITIONS_FILE
    positions_path.write_text(
        json.dumps(
            {
                s.id: [round(s.px[0], 1), round(s.px[1], 1)]
                for s in sorted(settlements, key=lambda s: s.id)
            },
            ensure_ascii=False,
            indent=1,
        )
        + "\n",
        encoding="utf-8",
    )
    graph_path = map_dir / GRAPH_FILE
    graph_path.write_text(
        json.dumps({"edges": [e.to_json() for e in edges]}, indent=1) + "\n",
        encoding="utf-8",
    )
    edge_paths_path, traced = edge_paths_step.write(
        map_dir, [e.to_json() for e in edges], positions, ROADS_FILE
    )
    render_preview(labels, settlements, edges, preview_path)

    snapped = [s for s in settlements if s.snapped]
    for settlement in snapped:
        warnings.append(
            f"{settlement.id} ({settlement.name}) hors de {settlement.province} : "
            f"position de jeu ramenée à {settlement.px[0]:.1f}, {settlement.px[1]:.1f}"
        )
    return SettlementResult(
        graph=graph_path,
        positions=positions_path,
        preview=preview_path,
        seconds=time.perf_counter() - started,
        edge_paths=edge_paths_path,
        traced_edges=traced,
        settlements=len(settlements),
        fallback_cities=[s.id for s in settlements if s.fallback],
        snapped=[s.id for s in snapped],
        land_edges=sum(not e.sea for e in edges),
        road_edges=sum(e.road for e in edges),
        sea_edges=sum(e.sea for e in edges),
        components=component_count(settlements, edges),
        warnings=warnings,
    )
