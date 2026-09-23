"""Province polygons, id raster and neighbour graph (``cent-ans geo provinces``).

Method (see ``docs/geo.md``, section « Provinces »):

1. Every province of ``data/provinces/`` provides a seed (``geo.seed_lonlat``)
   and a weight (``geo.voronoi_weight``). Seeds are projected to map pixels.
2. A *weighted cost-distance Voronoi* is computed on a 1024² work grid (4 px
   blocks of the 4096² map): the travel cost from each seed is propagated over
   land only (sea impassable, major rivers cost x4) with
   :class:`skimage.graph.MCP_Geometric`, divided by the seed weight, and each
   land pixel goes to the cheapest seed. Provinces therefore never jump across
   straits, and rivers act as soft borders.
3. The label map is upsampled (nearest) to 4096², land pixels left without a
   label (islands without seed, pixels lost by the downsampling) take the
   nearest labelled pixel (Euclidean), and borders are smoothed with a 5x5
   majority filter.
4. Regions are vectorised (``rasterio.features.shapes``), simplified (1.5 px)
   and cleaned of slivers; land neighbours come from adjacent pixel pairs and
   sea neighbours from the distance between coastal pixels.
"""

from __future__ import annotations

import json
import time
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
import rasterio.features
import shapely
from affine import Affine
from PIL import Image, ImageDraw
from scipy import ndimage
from scipy.spatial import cKDTree
from shapely.geometry import MultiPolygon, Polygon, shape
from skimage.filters import rank
from skimage.graph import MCP_Geometric
from skimage.morphology import footprint_rectangle

from cent_ans_tools.geo.project import MapGrid

REPO_DIR = Path(__file__).resolve().parents[3]
MAP_DIR = REPO_DIR / "data" / "map"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"
FACTIONS_DIR = REPO_DIR / "data" / "factions"
PREVIEW_PATH = REPO_DIR / "docs" / "img" / "provinces-preview.png"

WORK_FACTOR = 4  # cost distance is solved on a (SIZE_PX / WORK_FACTOR)² grid
RIVER_COST = 4.0  # crossing a major river costs four land pixels
RIVER_MAX_SCALERANK = 4
SIMPLIFY_TOLERANCE_PX = 1.5
MIN_PART_AREA_PX = 30.0
MIN_SHARED_BORDER_PX = 3
# A landmass without any seed larger than this (map pixels, ~33 000 km²) is
# left unassigned (0): continental Africa. Smaller ones (islands) join the
# nearest province.
MAX_SEEDLESS_LANDMASS_PX = 64_000
SEA_NEIGHBOUR_RADIUS_PX = 60.0
COAST_SAMPLE_STEP = 4
MAJORITY_FOOTPRINT = 5


@dataclass(frozen=True)
class ProvinceSeed:
    """Gameplay fields of a province needed by the geo pipeline."""

    id: str
    name: str
    owner: str
    seed_lonlat: tuple[float, float]
    capital_lonlat: tuple[float, float]
    weight: float


@dataclass
class ProvinceResult:
    """Outputs of :func:`build`, plus diagnostics for the report."""

    geojson: Path
    id_raster: Path
    preview: Path
    seconds: float
    snapped_capitals: list[str] = field(default_factory=list)
    snapped_seeds: list[str] = field(default_factory=list)
    neighbour_counts: dict[str, int] = field(default_factory=dict)
    sea_neighbour_counts: dict[str, int] = field(default_factory=dict)


# ----------------------------------------------------------------------------
# Inputs
# ----------------------------------------------------------------------------


def load_seeds(provinces_dir: Path = PROVINCES_DIR) -> list[ProvinceSeed]:
    """Read every ``data/provinces/*.json`` sorted by id."""
    seeds = []
    for path in sorted(provinces_dir.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        geo = data["geo"]
        seeds.append(
            ProvinceSeed(
                id=data["id"],
                name=data["name"]["display"],
                owner=data["owner"],
                seed_lonlat=tuple(geo["seed_lonlat"]),
                capital_lonlat=tuple(geo["capital_lonlat"]),
                weight=float(geo.get("voronoi_weight", 1.0)),
            )
        )
    seeds.sort(key=lambda seed: seed.id)
    return seeds


def load_faction_colours(factions_dir: Path = FACTIONS_DIR) -> dict[str, str]:
    """Map faction id -> ``heraldry.primary_color`` (hex)."""
    colours = {}
    for path in sorted(factions_dir.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        colours[data["id"]] = data.get("heraldry", {}).get("primary_color", "#888888")
    return colours


def load_grid(map_dir: Path = MAP_DIR) -> MapGrid:
    """The grid described by ``map.json`` (bounds and size)."""
    metadata = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    return MapGrid(tuple(metadata["bounds_projected"]), metadata["size_px"][0])


def load_land_mask(map_dir: Path = MAP_DIR) -> np.ndarray:
    """``land_mask.png`` as a boolean array."""
    with Image.open(map_dir / "land_mask.png") as image:
        return np.asarray(image) > 0


def river_mask(
    map_dir: Path, size: int, factor: int, max_scalerank: int = RIVER_MAX_SCALERANK
) -> np.ndarray:
    """Rasterise major rivers (``scalerank <= max_scalerank``) on the work grid."""
    rivers = json.loads((map_dir / "rivers.geojson").read_text(encoding="utf-8"))
    shapes = [
        (feature["geometry"], 1)
        for feature in rivers["features"]
        if (feature["properties"].get("scalerank") or 99) <= max_scalerank
    ]
    if not shapes:
        return np.zeros((size, size), dtype=bool)
    raster = rasterio.features.rasterize(
        shapes,
        out_shape=(size, size),
        transform=Affine(factor, 0, 0, 0, factor, 0),
        fill=0,
        all_touched=True,
        dtype="uint8",
    )
    return raster > 0


# ----------------------------------------------------------------------------
# Weighted cost-distance Voronoi
# ----------------------------------------------------------------------------


def downsample_mask(mask: np.ndarray, factor: int) -> np.ndarray:
    """Block-majority downsampling of a boolean mask."""
    size = mask.shape[0] // factor
    blocks = mask.reshape(size, factor, size, factor).mean(axis=(1, 3))
    return blocks >= 0.5


def snap_to_mask(points: np.ndarray, mask: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Move integer ``(row, col)`` points that fall outside ``mask`` to its nearest cell.

    Returns:
        ``(snapped_points, moved)`` where ``moved`` flags the points that changed.
    """
    _, indices = ndimage.distance_transform_edt(~mask, return_indices=True)
    rows = np.clip(points[:, 0], 0, mask.shape[0] - 1)
    cols = np.clip(points[:, 1], 0, mask.shape[1] - 1)
    snapped = np.column_stack([indices[0][rows, cols], indices[1][rows, cols]])
    moved = np.any(snapped != points, axis=1)
    return snapped, moved


def weighted_cost_voronoi(
    land: np.ndarray,
    rivers: np.ndarray,
    sources_rc: list[np.ndarray],
    weights: np.ndarray,
    river_cost: float = RIVER_COST,
) -> np.ndarray:
    """Assign each land cell to the province with the lowest ``cost_distance / weight``.

    Args:
        land: Boolean work grid (True = passable).
        rivers: Boolean work grid of major rivers (cost multiplier).
        sources_rc: Per province, an ``(k, 2)`` integer ``(row, col)`` array of
            source cells (seed and capital), all on land.
        weights: ``(n,)`` positive weights (bigger = larger province).
        river_cost: Cost of a river cell relative to a plain land cell.

    Returns:
        ``uint16`` label map, 1-based province index, 0 where unreachable or sea.
    """
    costs = np.where(land, np.where(rivers, river_cost, 1.0), np.inf).astype(np.float64)
    best_cost = np.full(land.shape, np.inf, dtype=np.float64)
    labels = np.zeros(land.shape, dtype=np.uint16)
    for index, (sources, weight) in enumerate(
        zip(sources_rc, weights, strict=True), start=1
    ):
        mcp = MCP_Geometric(costs, fully_connected=True)
        cost, _ = mcp.find_costs([(int(row), int(col)) for row, col in sources])
        scaled = cost / weight
        better = scaled < best_cost
        best_cost[better] = scaled[better]
        labels[better] = index
    labels[~land] = 0
    return labels


def fill_unlabelled(labels: np.ndarray, land: np.ndarray) -> np.ndarray:
    """Give every land pixel without a label the label of the nearest labelled pixel."""
    _, indices = ndimage.distance_transform_edt(labels == 0, return_indices=True)
    filled = labels[indices[0], indices[1]]
    filled[~land] = 0
    return filled


def assignable_land(
    land: np.ndarray, sources_rc: np.ndarray, max_seedless_px: int
) -> np.ndarray:
    """Land minus the seedless landmasses larger than ``max_seedless_px``.

    Connected components (8-connectivity) of ``land`` that contain no source
    pixel and exceed the threshold (continental Africa) are excluded so they
    are never filled by the nearest province.
    """
    components, count = ndimage.label(land, structure=np.ones((3, 3)))
    sizes = np.bincount(components.ravel(), minlength=count + 1)
    seeded = np.zeros(count + 1, dtype=bool)
    seeded[components[sources_rc[:, 0], sources_rc[:, 1]]] = True
    excluded = (~seeded) & (sizes > max_seedless_px)
    excluded[0] = False
    return land & ~excluded[components]


def smooth_labels(labels: np.ndarray, land: np.ndarray, size: int) -> np.ndarray:
    """Majority filter that ignores the sea (sea is filled first, masked after)."""
    _, indices = ndimage.distance_transform_edt(labels == 0, return_indices=True)
    full = labels[indices[0], indices[1]]
    smoothed = rank.majority(full, footprint_rectangle((size, size)))
    smoothed[~land] = 0
    return smoothed.astype(np.uint16)


def upsample_nearest(labels: np.ndarray, factor: int) -> np.ndarray:
    """Repeat each cell ``factor`` times in both directions."""
    return np.repeat(np.repeat(labels, factor, axis=0), factor, axis=1)


# ----------------------------------------------------------------------------
# Graph and vectors
# ----------------------------------------------------------------------------


def encode_ids(labels: np.ndarray) -> np.ndarray:
    """``province_ids.png`` encoding: R = idx & 255, G = idx >> 8, B = 0."""
    rgb = np.zeros((*labels.shape, 3), dtype=np.uint8)
    rgb[..., 0] = labels & 255
    rgb[..., 1] = labels >> 8
    return rgb


def decode_ids(rgb: np.ndarray) -> np.ndarray:
    """Inverse of :func:`encode_ids`."""
    return rgb[..., 0].astype(np.uint16) | (rgb[..., 1].astype(np.uint16) << 8)


def land_neighbours(
    labels: np.ndarray, min_shared_px: int = MIN_SHARED_BORDER_PX
) -> dict[int, set[int]]:
    """Pairs of labels sharing at least ``min_shared_px`` adjacent pixel pairs."""
    pairs = []
    for a, b in (
        (labels[:, :-1], labels[:, 1:]),
        (labels[:-1, :], labels[1:, :]),
    ):
        keep = (a != b) & (a > 0) & (b > 0)
        low = np.minimum(a[keep], b[keep]).astype(np.int64)
        high = np.maximum(a[keep], b[keep]).astype(np.int64)
        pairs.append(low * 65536 + high)
    keys, counts = np.unique(np.concatenate(pairs), return_counts=True)
    graph: dict[int, set[int]] = {}
    for key, count in zip(keys, counts, strict=True):
        if count < min_shared_px:
            continue
        low, high = int(key // 65536), int(key % 65536)
        graph.setdefault(low, set()).add(high)
        graph.setdefault(high, set()).add(low)
    return graph


def coastal_pixels(labels: np.ndarray) -> np.ndarray:
    """``(n, 3)`` array of ``(row, col, label)`` for land pixels touching the sea."""
    land = labels > 0
    padded = np.pad(land, 1, constant_values=True)
    sea_touch = (
        ~padded[:-2, 1:-1] | ~padded[2:, 1:-1] | ~padded[1:-1, :-2] | ~padded[1:-1, 2:]
    )
    rows, cols = np.nonzero(land & sea_touch)
    return np.column_stack([rows, cols, labels[rows, cols]])


def sea_neighbours(
    labels: np.ndarray,
    land_graph: dict[int, set[int]],
    radius_px: float = SEA_NEIGHBOUR_RADIUS_PX,
    step: int = COAST_SAMPLE_STEP,
) -> dict[int, set[int]]:
    """Coastal provinces whose coasts lie within ``radius_px`` and are not land neighbours."""
    coast = coastal_pixels(labels)[::step]
    tree = cKDTree(coast[:, :2].astype(np.float64))
    pairs = tree.query_pairs(radius_px, output_type="ndarray")
    a = coast[pairs[:, 0], 2]
    b = coast[pairs[:, 1], 2]
    keep = a != b
    graph: dict[int, set[int]] = {}
    for low, high in {
        (int(min(x, y)), int(max(x, y))) for x, y in zip(a[keep], b[keep], strict=True)
    }:
        if high in land_graph.get(low, set()):
            continue
        graph.setdefault(low, set()).add(high)
        graph.setdefault(high, set()).add(low)
    return graph


def vectorise(
    labels: np.ndarray,
    tolerance: float = SIMPLIFY_TOLERANCE_PX,
    min_area: float = MIN_PART_AREA_PX,
) -> dict[int, MultiPolygon | Polygon]:
    """Polygon per label in pixel coordinates (pixel corners), simplified and cleaned."""
    parts: dict[int, list[Polygon]] = {}
    for geometry, value in rasterio.features.shapes(
        labels.astype(np.int32), mask=labels > 0, connectivity=4
    ):
        parts.setdefault(int(value), []).append(shape(geometry))
    result = {}
    for value, polygons in parts.items():
        merged = shapely.unary_union(polygons).simplify(
            tolerance, preserve_topology=True
        )
        pieces = list(merged.geoms) if isinstance(merged, MultiPolygon) else [merged]
        kept = [piece for piece in pieces if piece.area >= min_area]
        if not kept:
            kept = [max(pieces, key=lambda piece: piece.area)]
        result[value] = kept[0] if len(kept) == 1 else MultiPolygon(kept)
    return result


def representative_centroid(geometry: Polygon | MultiPolygon) -> tuple[float, float]:
    """Centroid of the largest part when it falls inside it, else a point inside."""
    largest = (
        max(geometry.geoms, key=lambda part: part.area)
        if isinstance(geometry, MultiPolygon)
        else geometry
    )
    centroid = largest.centroid
    point = centroid if largest.contains(centroid) else largest.representative_point()
    return float(point.x), float(point.y)


def snap_capital(
    labels: np.ndarray, index: int, px: float, py: float
) -> tuple[tuple[float, float], bool]:
    """Keep the capital pixel if it lies in its province, else move it to the nearest one."""
    row, col = int(py), int(px)
    inside = 0 <= row < labels.shape[0] and 0 <= col < labels.shape[1]
    if inside and labels[row, col] == index:
        return (px, py), False
    rows, cols = np.nonzero(labels == index)
    nearest = np.argmin((rows - py) ** 2 + (cols - px) ** 2)
    return (float(cols[nearest]) + 0.5, float(rows[nearest]) + 0.5), True


def round_coords(geometry: Polygon | MultiPolygon, decimals: int = 1) -> dict:
    """GeoJSON mapping with rounded coordinates."""
    rounded = shapely.transform(geometry, lambda coords: np.round(coords, decimals))
    return shapely.geometry.mapping(rounded)


# ----------------------------------------------------------------------------
# Preview
# ----------------------------------------------------------------------------


def _hex_to_rgb(value: str) -> tuple[int, int, int]:
    value = value.lstrip("#")
    return tuple(int(value[i : i + 2], 16) for i in (0, 2, 4))  # type: ignore[return-value]


def render_preview(
    labels: np.ndarray,
    seeds: list[ProvinceSeed],
    capitals_px: list[tuple[float, float]],
    colours: dict[str, str],
    path: Path,
    map_dir: Path = MAP_DIR,
    size: int = 1024,
) -> None:
    """Owner-coloured provinces over a hillshade, black borders, capital dots."""
    from cent_ans_tools.geo import build, terrain

    factor = labels.shape[0] // size
    small = labels[::factor, ::factor]
    palette = np.zeros((len(seeds) + 1, 3), dtype=np.float64)
    palette[0] = (0.45, 0.60, 0.78)
    for index, seed in enumerate(seeds, start=1):
        palette[index] = np.array(_hex_to_rgb(colours.get(seed.owner, "#888888"))) / 255
    rgb = palette[small]
    heightmap = map_dir / "heightmap.png"
    if heightmap.exists():
        heights = terrain.uint16_to_height(terrain.read_png16(heightmap))
        small_height = heights.reshape(size, factor, size, factor).mean(axis=(1, 3))
        shade = build.hillshade(small_height, load_grid(map_dir).meters_per_px * factor)
        rgb = rgb * (0.6 + 0.4 * shade[..., None])
    border = np.zeros(small.shape, dtype=bool)
    border[:, :-1] |= small[:, :-1] != small[:, 1:]
    border[:-1, :] |= small[:-1, :] != small[1:, :]
    rgb[border] = 0.0
    image = Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8), mode="RGB")
    draw = ImageDraw.Draw(image)
    for px, py in capitals_px:
        x, y = px / factor, py / factor
        draw.ellipse(
            (x - 2.5, y - 2.5, x + 2.5, y + 2.5),
            fill=(255, 255, 255),
            outline=(0, 0, 0),
        )
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)


# ----------------------------------------------------------------------------
# Orchestration
# ----------------------------------------------------------------------------


def build(
    map_dir: Path = MAP_DIR,
    provinces_dir: Path = PROVINCES_DIR,
    factions_dir: Path = FACTIONS_DIR,
    preview_path: Path = PREVIEW_PATH,
) -> ProvinceResult:
    """Generate ``provinces.geojson``, ``province_ids.png`` and the preview."""
    started = time.perf_counter()
    grid = load_grid(map_dir)
    seeds = load_seeds(provinces_dir)
    land = load_land_mask(map_dir)
    size = grid.size_px
    work_size = size // WORK_FACTOR

    def to_rows_cols(lonlat: list[tuple[float, float]]) -> np.ndarray:
        px, py = grid.lonlat_to_pixel(
            np.array([point[0] for point in lonlat]),
            np.array([point[1] for point in lonlat]),
        )
        return np.column_stack([py, px]).astype(np.int64)

    seeds_rc = to_rows_cols([seed.seed_lonlat for seed in seeds])
    capitals_rc = to_rows_cols([seed.capital_lonlat for seed in seeds])
    weights = np.array([seed.weight for seed in seeds])

    work_land = downsample_mask(land, WORK_FACTOR)
    rivers = river_mask(map_dir, work_size, WORK_FACTOR)
    work_seeds, moved = snap_to_mask(seeds_rc // WORK_FACTOR, work_land)
    snapped_seeds = [seeds[i].id for i in np.nonzero(moved)[0]]
    work_capitals, _ = snap_to_mask(capitals_rc // WORK_FACTOR, work_land)
    sources = [
        np.array([seed, capital])
        for seed, capital in zip(work_seeds, work_capitals, strict=True)
    ]

    work_labels = weighted_cost_voronoi(work_land, rivers, sources, weights)
    labels = upsample_nearest(work_labels, WORK_FACTOR)
    land = assignable_land(
        land, np.clip(seeds_rc, 0, size - 1), MAX_SEEDLESS_LANDMASS_PX
    )
    labels[~land] = 0
    labels = fill_unlabelled(labels, land)
    labels = smooth_labels(labels, land, MAJORITY_FOOTPRINT)
    labels = fill_unlabelled(labels, land)

    id_raster_path = map_dir / "province_ids.png"
    Image.fromarray(encode_ids(labels), mode="RGB").save(
        id_raster_path, compress_level=9
    )

    land_graph = land_neighbours(labels)
    sea_graph = sea_neighbours(labels, land_graph)
    geometries = vectorise(labels)

    cap_lon = np.array([seed.capital_lonlat[0] for seed in seeds])
    cap_lat = np.array([seed.capital_lonlat[1] for seed in seeds])
    cap_px, cap_py = grid.lonlat_to_pixel(cap_lon, cap_lat)

    features = []
    capitals = []
    snapped_capitals = []
    for index, seed in enumerate(seeds, start=1):
        geometry = geometries[index]
        capital, moved_capital = snap_capital(
            labels, index, float(cap_px[index - 1]), float(cap_py[index - 1])
        )
        if moved_capital:
            snapped_capitals.append(seed.id)
        capitals.append(capital)
        centroid = representative_centroid(geometry)
        features.append(
            {
                "type": "Feature",
                "properties": {
                    "id": seed.id,
                    "index": index,
                    "name": seed.name,
                    "owner": seed.owner,
                    "centroid": [round(centroid[0], 1), round(centroid[1], 1)],
                    "capital_px": [round(capital[0], 1), round(capital[1], 1)],
                    "neighbors": sorted(
                        seeds[j - 1].id for j in land_graph.get(index, ())
                    ),
                    "sea_neighbors": sorted(
                        seeds[j - 1].id for j in sea_graph.get(index, ())
                    ),
                    "area_px": int(np.count_nonzero(labels == index)),
                },
                "geometry": round_coords(geometry),
            }
        )
    geojson_path = map_dir / "provinces.geojson"
    geojson_path.write_text(
        json.dumps(
            {"type": "FeatureCollection", "features": features}, separators=(",", ":")
        )
        + "\n",
        encoding="utf-8",
    )

    render_preview(
        labels,
        seeds,
        capitals,
        load_faction_colours(factions_dir),
        preview_path,
        map_dir,
    )
    return ProvinceResult(
        geojson=geojson_path,
        id_raster=id_raster_path,
        preview=preview_path,
        seconds=time.perf_counter() - started,
        snapped_capitals=snapped_capitals,
        snapped_seeds=snapped_seeds,
        neighbour_counts={
            seed.id: len(land_graph.get(i, ())) for i, seed in enumerate(seeds, start=1)
        },
        sea_neighbour_counts={
            seed.id: len(sea_graph.get(i, ())) for i, seed in enumerate(seeds, start=1)
        },
    )
