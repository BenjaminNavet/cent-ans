"""Lake outlines for the campaign water meshes (lot SS3, ADR 0142 §4).

A lake is a component of inland water of ``data/map/land_mask.png``: water
not connected to the image border (the sea), of at least ``min_area_px``
pixels, whose water level is above sea level (below it, the sea plane of
``water.gdshader`` already covers it: Caspian, closed-off seas, polders).
Modern dam reservoirs are left out: ``modern_reservoirs.json`` (ADR 0036)
and the Natural Earth ``Reservoir`` features (Volga, Dnieper cascades...).

For each lake the tool writes, in map pixels (1 px = 1 world unit, 719 m;
pixel ``i`` covers ``[i, i + 1]``, centred at ``i + 0.5`` as in ``MapData``)::

    {"id", "name", "level_m", "area_km2", "center_px": [x, y],
     "islands", "polygon_px": [[x, y], ...]}

``polygon_px`` is the simplified outer contour (marching squares at the
water/land edge, Douglas-Peucker preserving topology); islands are counted
but not cut out (the island ground, above the water, hides the sheet).
``level_m`` is a most frequent height of the rendered heightmap
(``heightmap_render.png``, the surface the game draws): Copernicus flattens
lakes to one value (same reasoning as :mod:`cent_ans_tools.geo.surface`).
A water component holding several levels (lake chains) is split into flat
basins, one lake each; water that is flat nowhere (a reconstructed reservoir
valley) is skipped (:func:`flat_basins`). ``name`` comes from Natural Earth lakes
(French name first) when the raw layer is present, else it is empty.
Rendering only (``lakes_renderer.gd``), no game rule.
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from shapely.geometry import MultiPolygon, Polygon
from skimage import measure

from cent_ans_tools.geo import project

DATA = Path(__file__).resolve().parents[3] / "data"
MAP_DIR = DATA / "map"
OUTPUT_FILE = MAP_DIR / "lakes.json"
RESERVOIRS_FILE = MAP_DIR / "modern_reservoirs.json"
NATURAL_EARTH_LAKES = (
    Path(__file__).resolve().parents[2]
    / "geo"
    / "raw"
    / "natural_earth"
    / "ne_10m_lakes"
    / "ne_10m_lakes.shp"
)

Image.MAX_IMAGE_PIXELS = None

#: Natural lakes that Natural Earth files as ``Reservoir`` because a modern dam
#: regulates them: they existed in 1340 and stay lakes (names as printed).
NATURAL_REGULATED_LAKES = frozenset(
    {
        "Ilmen",
        "Imandra",
        "Kemijärvi",
        "Koitere",
        "Koubenskoïe",
        "Kovdozero",
        "Mjøsa",
        "Ondosero",
        "Oulu",
        "Oumbozero",
        "Rikkavesi",
        "Stora Lulevatten",
    }
)


@dataclass(frozen=True)
class LakesParams:
    """Extraction settings (all distances in map pixels)."""

    #: Smallest lake kept (30 px ≈ 15 km²).
    min_area_px: int = 30
    #: Douglas-Peucker tolerance of the outline.
    simplify_px: float = 0.6
    #: Lakes at or below this level are left to the sea plane.
    min_level_m: float = 0.5
    #: Height tolerance of a flat basin (see :func:`flat_basins`).
    flat_tolerance_m: float = 3.0
    #: Most basins (water levels) cut out of one water component.
    max_basins: int = 8
    #: Search radius around a reservoir point (the point is on the water).
    reservoir_radius_px: int = 3
    #: Search radius around a Natural Earth label point.
    name_radius_px: int = 2


@dataclass
class LakesResult:
    """Summary of a run."""

    output: Path
    lakes: int = 0
    vertices: int = 0
    excluded_reservoirs: list[str] = field(default_factory=list)
    below_sea: int = 0
    not_flat: int = 0
    named: int = 0


def inland_water_labels(land: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """Label inland water components (4-connectivity).

    Args:
        land: Boolean land mask (``True`` = land), shape ``(rows, cols)``.

    Returns:
        ``(labels, sizes)``: labels of the water components (0 = land) with
        every component touching the image border reset to 0, and the pixel
        count of each label.
    """
    labels, _ = ndimage.label(~land)
    border = np.unique(
        np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    )
    labels[np.isin(labels, border[border > 0])] = 0
    sizes = np.bincount(labels.ravel())
    sizes[0] = 0
    return labels, sizes


def lake_level(
    heights: np.ndarray, component: np.ndarray, tolerance_m: float = 3.0
) -> tuple[float, float]:
    """Water level (m) and flatness of a lake.

    Args:
        heights: Heights in metres, same shape as ``component``.
        component: Boolean mask of the lake.
        tolerance_m: Height tolerance of the flatness measure.

    Returns:
        ``(level, flat)``: the modal height (0.1 m bins) inside the component
        eroded by one pixel (the component itself when erosion empties it),
        and the share of those pixels within ``tolerance_m`` of it.
    """
    inner = ndimage.binary_erosion(component)
    values = heights[inner if inner.any() else component]
    bins = np.round(values * 10.0).astype(np.int64)
    uniq, counts = np.unique(bins, return_counts=True)
    level = float(uniq[counts.argmax()]) / 10.0
    return level, float(np.mean(np.abs(values - level) <= tolerance_m))


def flat_basins(
    heights: np.ndarray, component: np.ndarray, params: LakesParams
) -> list[tuple[float, np.ndarray]]:
    """Split a water component into flat basins, one per water level.

    A component of the land mask can hold several water levels (the Saimaa
    and Päijänne systems, chains of lakes joined by rivers) or none at all
    (a modern reservoir whose valley the rendered heightmap reconstructs).
    Repeatedly: the modal height of the remaining pixels is a level; its core
    is the remaining pixels within ``flat_tolerance_m`` of it; each connected
    core of at least ``min_area_px`` pixels is a basin, grown by two pixels
    over the component where the ground is not lower than the level (blended
    shore pixels, so the outline reaches the bank). Sloped water yields only
    small cores and no basin.

    Returns:
        ``[(level, mask)]`` with ``mask`` the grown basin (same shape).
    """
    remaining = component.copy()
    basins: list[tuple[float, np.ndarray]] = []
    tol = params.flat_tolerance_m
    for _ in range(params.max_basins):
        if remaining.sum() < params.min_area_px:
            break
        values = heights[remaining]
        bins = np.round(values * 10.0).astype(np.int64)
        uniq, counts = np.unique(bins, return_counts=True)
        level = float(uniq[counts.argmax()]) / 10.0
        core = remaining & (np.abs(heights - level) <= tol)
        remaining &= ~core
        cores, count = ndimage.label(core)
        sizes = np.bincount(cores.ravel())
        # A basin is a solid area: shore rings of blended pixels and the thin
        # bands of a sloped valley vanish under a one-pixel erosion.
        big = [
            index
            for index in range(1, count + 1)
            if sizes[index] * 2 >= params.min_area_px
            and ndimage.binary_erosion(cores == index).sum() * 8
            >= params.min_area_px
        ]
        if not big and core.sum() * 2 < params.min_area_px:
            break
        rim = component & (heights >= level - tol)
        for index in big:
            grown = ndimage.binary_dilation(cores == index, iterations=2) & rim
            basins.append((level, grown))
    return basins


def outline(component: np.ndarray, simplify_px: float) -> tuple[Polygon, int]:
    """Simplified outer outline of a component in local pixel coordinates.

    Args:
        component: Boolean mask (a crop around one lake).
        simplify_px: Douglas-Peucker tolerance.

    Returns:
        ``(polygon, islands)``: polygon in ``(x, y)`` pixel-edge coordinates
        of the crop (pixel ``i`` centred at ``i + 0.5``), and the number of
        interior contours (islands).
    """
    padded = np.pad(component.astype(np.float32), 1)
    contours = measure.find_contours(padded, 0.5)
    rings = []
    for contour in contours:
        # (row, col) of the padded crop -> (x, y) of the crop, pixel centres at +0.5.
        xy = np.column_stack([contour[:, 1] - 0.5, contour[:, 0] - 0.5])
        if len(xy) >= 4:
            rings.append(Polygon(xy))
    rings.sort(key=lambda ring: ring.area, reverse=True)
    shape = rings[0].buffer(0)
    if isinstance(shape, MultiPolygon):
        shape = max(shape.geoms, key=lambda part: part.area)
    simplified = shape.simplify(simplify_px, preserve_topology=True)
    if isinstance(simplified, MultiPolygon):
        simplified = max(simplified.geoms, key=lambda part: part.area)
    return Polygon(simplified.exterior), max(len(rings) - 1, 0)


def _label_near(labels: np.ndarray, px: float, py: float, radius: int) -> int:
    """Most frequent non-zero label within ``radius`` of a pixel (0 if none)."""
    rows, cols = labels.shape
    x, y = int(np.floor(px)), int(np.floor(py))
    x0, x1 = max(x - radius, 0), min(x + radius + 1, cols)
    y0, y1 = max(y - radius, 0), min(y + radius + 1, rows)
    if x0 >= x1 or y0 >= y1:
        return 0
    window = labels[y0:y1, x0:x1]
    found = window[window > 0]
    if found.size == 0:
        return 0
    values, counts = np.unique(found, return_counts=True)
    return int(values[counts.argmax()])


def reservoir_labels(
    labels: np.ndarray,
    grid: project.MapGrid,
    reservoirs: list[dict],
    radius: int,
) -> dict[int, str]:
    """Labels of the components holding a modern reservoir (label -> reservoir id)."""
    result: dict[int, str] = {}
    for reservoir in reservoirs:
        px, py = grid.lonlat_to_pixel(reservoir["lon"], reservoir["lat"])
        label = _label_near(labels, float(px), float(py), radius)
        if label:
            result[label] = reservoir["id"]
    return result


def natural_earth_lookup(
    labels: np.ndarray, grid: project.MapGrid, shapefile: Path, radius: int
) -> tuple[dict[int, str], dict[int, str]]:
    """Names and reservoirs of the components from Natural Earth lakes.

    The largest Natural Earth feature whose label point falls on a component
    decides: a lake gives its name (French first), a ``Reservoir`` (dam lake,
    mostly 20th century: Rybinsk, Kuibyshev...) marks the component as modern.

    Returns:
        ``(names, reservoirs)``: label -> name, label -> ``ne:<name>``.
    """
    if not shapefile.exists():
        return {}, {}
    import geopandas as gpd

    frame = gpd.read_file(shapefile)
    frame = frame.assign(_area=frame.geometry.area).sort_values(
        "_area", ascending=False
    )
    names: dict[int, str] = {}
    reservoirs: dict[int, str] = {}
    for _, row in frame.iterrows():
        name = row.get("name_fr") or row.get("name") or ""
        name = name if isinstance(name, str) else ""
        point = row.geometry.representative_point()
        px, py = grid.lonlat_to_pixel(point.x, point.y)
        label = _label_near(labels, float(px), float(py), radius)
        if not label or label in names or label in reservoirs:
            continue
        if row.get("featurecla") == "Reservoir" and name not in NATURAL_REGULATED_LAKES:
            reservoirs[label] = f"ne:{name or row.get('ne_id')}"
        elif name:
            names[label] = name
    return names, reservoirs


def extract(
    land: np.ndarray,
    heights: np.ndarray,
    meters_per_px: float,
    params: LakesParams,
    excluded: dict[int, str] | None = None,
    names: dict[int, str] | None = None,
    labels: tuple[np.ndarray, np.ndarray] | None = None,
) -> tuple[list[dict], int]:
    """Extract the lakes of a land mask.

    Args:
        land: Boolean land mask.
        heights: Heights in metres (same shape).
        meters_per_px: Pixel size (area in km²).
        params: Extraction settings.
        excluded: Labels to skip (reservoirs), from the same labelling.
        names: Label -> name.
        labels: Precomputed :func:`inland_water_labels` result.

    Returns:
        ``(lakes, skipped)``: lake records sorted by decreasing area, and the
        number of components skipped as ``{"below_sea", "not_flat"}``.
    """
    lab, sizes = labels if labels is not None else inland_water_labels(land)
    excluded = excluded or {}
    names = names or {}
    boxes = ndimage.find_objects(lab)
    lakes: list[dict] = []
    skipped = {"below_sea": 0, "not_flat": 0}
    for label in np.flatnonzero(sizes >= params.min_area_px):
        if label in excluded:
            continue
        box = boxes[label - 1]
        component = lab[box] == label
        level, _ = lake_level(heights[box], component, params.flat_tolerance_m)
        if level <= params.min_level_m:
            skipped["below_sea"] += 1
            continue
        basins = [
            (basin_level, mask)
            for basin_level, mask in flat_basins(heights[box], component, params)
            if basin_level > params.min_level_m
        ]
        if not basins:
            skipped["not_flat"] += 1
            continue
        basins.sort(key=lambda basin: -int(basin[1].sum()))
        x0, y0 = box[1].start, box[0].start
        for rank, (basin_level, mask) in enumerate(basins):
            polygon, islands = outline(mask, params.simplify_px)
            coords = np.asarray(polygon.exterior.coords)[:-1] + (x0, y0)
            if len(coords) < 3:
                continue
            cy, cx = ndimage.center_of_mass(mask)
            area = float(mask.sum()) * meters_per_px**2 / 1e6
            lakes.append(
                {
                    "id": "",
                    "name": names.get(int(label), "") if rank == 0 else "",
                    "level_m": round(basin_level, 1),
                    "area_km2": round(area, 1),
                    "center_px": [round(cx + x0 + 0.5, 2), round(cy + y0 + 0.5, 2)],
                    "islands": islands,
                    "polygon_px": [[round(x, 2), round(y, 2)] for x, y in coords],
                }
            )
    lakes.sort(key=lambda lake: -lake["area_km2"])
    for index, lake in enumerate(lakes):
        lake["id"] = f"lake_{index:03d}"
    return lakes, skipped


def build(
    output: Path = OUTPUT_FILE,
    params: LakesParams | None = None,
    natural_earth: Path = NATURAL_EARTH_LAKES,
) -> LakesResult:
    """Write ``data/map/lakes.json`` from the land mask and rendered heightmap."""
    params = params or LakesParams()
    meta = json.loads((MAP_DIR / "map.json").read_text(encoding="utf-8"))
    grid = project.grid_from_metadata(meta)
    land = np.asarray(Image.open(MAP_DIR / "land_mask.png")) >= 128
    render = meta.get("render_heightmap", {}).get("file", "heightmap.png")
    raw = np.asarray(Image.open(MAP_DIR / render)).astype(np.float64)
    h_min, h_max = meta["height_min_m"], meta["height_max_m"]
    heights = h_min + raw / 65535.0 * (h_max - h_min)
    labelled = inland_water_labels(land)
    reservoirs = json.loads(RESERVOIRS_FILE.read_text(encoding="utf-8"))["reservoirs"]
    excluded = reservoir_labels(
        labelled[0], grid, reservoirs, params.reservoir_radius_px
    )
    excluded = {
        label: rid
        for label, rid in excluded.items()
        if labelled[1][label] >= params.min_area_px
    }
    names, ne_reservoirs = natural_earth_lookup(
        labelled[0], grid, natural_earth, params.name_radius_px
    )
    for label, rid in ne_reservoirs.items():
        if labelled[1][label] >= params.min_area_px:
            excluded.setdefault(label, rid)
    lakes, skipped = extract(
        land,
        heights,
        grid.meters_per_px,
        params,
        excluded=excluded,
        names=names,
        labels=labelled,
    )
    document = {
        "description": (
            "Lacs de la carte de campagne (lot SS3, ADR 0142) : eau intérieure de "
            "land_mask.png non reliée à la mer, au-dessus du niveau de la mer, hors "
            "retenues modernes ; contour simplifié en px carte, niveau en mètres "
            f"({render}). Généré par `cent-ans geo lakes`. Rendu seulement."
        ),
        "params": {
            "min_area_px": params.min_area_px,
            "simplify_px": params.simplify_px,
            "min_level_m": params.min_level_m,
            "flat_tolerance_m": params.flat_tolerance_m,
            "max_basins": params.max_basins,
        },
        "lakes": lakes,
    }
    output.write_text(
        json.dumps(document, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    return LakesResult(
        output=output,
        lakes=len(lakes),
        vertices=sum(len(lake["polygon_px"]) for lake in lakes),
        excluded_reservoirs=sorted(excluded.values()),
        below_sea=skipped["below_sea"],
        not_flat=skipped["not_flat"],
        named=sum(1 for lake in lakes if lake["name"]),
    )
