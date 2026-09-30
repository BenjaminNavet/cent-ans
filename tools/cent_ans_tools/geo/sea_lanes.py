"""Sea lane geometry (lot SL1, ADR 0139): ``data/map/sea_lanes_px.json``.

Each lane of ``data/naval/sea_lanes.json`` links two port settlements. The
tool routes it over open water on a downsampled ``land_mask.png`` (least-cost
path, :func:`skimage.graph.route_through_array`), keeping away from the shore
(open-sea lanes more than coastal ones), then simplifies the path while it
stays on water and smooths it (Chaikin). Output, in map pixels (the
coordinate system of ``settlements_px.json``)::

    {"description": ..., "lanes": [{"id", "from", "to", "points": [[x, y], ...],
                                    "length_km"}]}

The first and last points are the ports themselves: a port up an estuary
(London, Bordeaux, Nantes) joins the sea by the shortest way. Rendering reads
the points; the core reads ``length_km`` (movement cost of the lane).
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from skimage.graph import route_through_array

DATA = Path(__file__).resolve().parents[3] / "data"
LANES_FILE = DATA / "naval" / "sea_lanes.json"
OUTPUT_FILE = DATA / "map" / "sea_lanes_px.json"
SETTLEMENTS_PX = DATA / "map" / "settlements_px.json"
LAND_MASK = DATA / "map" / "land_mask.png"
MAP_META = DATA / "map" / "map.json"

#: Map pixels per routing cell (4 × 719 m ≈ 2.9 km).
CELL = 4
#: Search radius (cells) for the sea cell nearest to a port.
PORT_SEARCH_CELLS = 40
#: Shore penalty: extra cost of a cell ``d`` cells from land is
#: ``weight / (1 + d)``; open-sea lanes keep further off than coastal ones.
SHORE_WEIGHT = {"coastal": 1.5, "open_sea": 6.0}
#: Douglas-Peucker tolerance (cells) of the simplified path.
SIMPLIFY_CELLS = 3.0
#: Chaikin iterations of the rendered polyline.
SMOOTH_ITERATIONS = 2


@dataclass
class SeaLanesResult:
    """Summary of a :func:`build` run."""

    output: Path
    lanes: int = 0
    total_km: float = 0.0
    warnings: list[str] = field(default_factory=list)


#: Water bodies smaller than this share of the largest one (lakes, estuaries
#: cut off at the routing resolution) are not sea.
MIN_SEA_SHARE = 0.01


def water_cells(mask: np.ndarray) -> np.ndarray:
    """Boolean grid of the routing cells that are sea.

    A cell is water when most of its ``mask`` pixels are (255 = land); only
    the large connected water bodies count as sea.
    """
    h, w = mask.shape
    hc, wc = h // CELL, w // CELL
    blocks = (mask[: hc * CELL, : wc * CELL] > 127).reshape(hc, CELL, wc, CELL)
    water = blocks.mean(axis=(1, 3)) < 0.5
    labels, count = ndimage.label(water, structure=np.ones((3, 3)))
    if count == 0:
        return water
    sizes = np.bincount(labels.ravel())
    sizes[0] = 0
    seas = np.nonzero(sizes >= sizes.max() * MIN_SEA_SHARE)[0]
    return np.isin(labels, seas)


def nearest_water(water: np.ndarray, cell: tuple[int, int]) -> tuple[int, int] | None:
    """Water cell closest to ``cell`` (row, col) within :data:`PORT_SEARCH_CELLS`."""
    r0, c0 = cell
    r_lo, r_hi = (
        max(0, r0 - PORT_SEARCH_CELLS),
        min(water.shape[0], r0 + PORT_SEARCH_CELLS + 1),
    )
    c_lo, c_hi = (
        max(0, c0 - PORT_SEARCH_CELLS),
        min(water.shape[1], c0 + PORT_SEARCH_CELLS + 1),
    )
    rows, cols = np.nonzero(water[r_lo:r_hi, c_lo:c_hi])
    if rows.size == 0:
        return None
    d = (rows + r_lo - r0) ** 2 + (cols + c_lo - c0) ** 2
    i = int(np.argmin(d))
    return int(rows[i] + r_lo), int(cols[i] + c_lo)


def segment_on_water(water: np.ndarray, a: np.ndarray, b: np.ndarray) -> bool:
    """Whether the straight segment ``a``-``b`` (cells, row/col) stays on water."""
    n = int(max(abs(b[0] - a[0]), abs(b[1] - a[1])) * 2) + 2
    t = np.linspace(0.0, 1.0, n)
    rows = np.clip(np.rint(a[0] + (b[0] - a[0]) * t).astype(int), 0, water.shape[0] - 1)
    cols = np.clip(np.rint(a[1] + (b[1] - a[1]) * t).astype(int), 0, water.shape[1] - 1)
    return bool(water[rows, cols].all())


def simplify(path: np.ndarray, water: np.ndarray, tolerance: float) -> np.ndarray:
    """Douglas-Peucker that only drops points when the shortcut stays on water."""
    if len(path) < 3:
        return path
    keep = np.zeros(len(path), dtype=bool)
    keep[0] = keep[-1] = True
    stack = [(0, len(path) - 1)]
    while stack:
        i, j = stack.pop()
        if j <= i + 1:
            continue
        a, b = path[i], path[j]
        ab = b - a
        norm = math.hypot(ab[0], ab[1]) or 1.0
        seg = path[i + 1 : j]
        dist = np.abs(ab[0] * (seg[:, 1] - a[1]) - ab[1] * (seg[:, 0] - a[0])) / norm
        k = int(np.argmax(dist)) + i + 1
        if dist.max() > tolerance or not segment_on_water(water, a, b):
            keep[k] = True
            stack.append((i, k))
            stack.append((k, j))
    return path[keep]


def chaikin(points: np.ndarray, iterations: int) -> np.ndarray:
    """Chaikin corner cutting, ends kept."""
    for _ in range(iterations):
        if len(points) < 3:
            return points
        q = 0.75 * points[:-1] + 0.25 * points[1:]
        r = 0.25 * points[:-1] + 0.75 * points[1:]
        inner = np.empty((2 * (len(points) - 1), 2))
        inner[0::2], inner[1::2] = q, r
        points = np.vstack([points[:1], inner[1:-1], points[-1:]])
    return points


def route_lane(
    water: np.ndarray,
    shore: np.ndarray,
    start_px: list[float],
    end_px: list[float],
    kind: str,
) -> np.ndarray | None:
    """Polyline (map pixels, x/y) of one lane, or ``None`` when unreachable."""
    to_cell = lambda px: (int(px[1] // CELL), int(px[0] // CELL))  # noqa: E731
    start = nearest_water(water, to_cell(start_px))
    end = nearest_water(water, to_cell(end_px))
    if start is None or end is None:
        return None
    cost = np.where(water, 1.0 + SHORE_WEIGHT[kind] / (1.0 + shore), np.inf)
    try:
        cells, _ = route_through_array(
            cost, start, end, fully_connected=True, geometric=True
        )
    except ValueError:
        return None
    path = np.array(cells, dtype=np.float64)
    if not np.isfinite(cost[tuple(path.astype(int).T)]).all():
        return None
    path = simplify(path, water, SIMPLIFY_CELLS)
    # Cells (row, col) -> map pixels (x, y) at the cell centre.
    points = np.column_stack([(path[:, 1] + 0.5) * CELL, (path[:, 0] + 0.5) * CELL])
    points = np.vstack([np.array([start_px]), points, np.array([end_px])])
    # Smoothing cuts corners: keep the smoothest version whose open-sea part
    # (between the first and last routed cells) stays on water.
    for iterations in range(SMOOTH_ITERATIONS, 0, -1):
        smoothed = chaikin(points, iterations)
        inner = smoothed[1:-1]
        cells = np.column_stack([inner[:, 1] / CELL, inner[:, 0] / CELL])
        near_port = (np.hypot(*(inner - points[0]).T) < CELL * 4) | (
            np.hypot(*(inner - points[-1]).T) < CELL * 4
        )
        if all(
            segment_on_water(water, a, b)
            for a, b, skip in zip(cells[:-1], cells[1:], near_port[1:], strict=True)
            if not skip
        ):
            return smoothed
    return points


def build(lanes_file: Path = LANES_FILE, output: Path = OUTPUT_FILE) -> SeaLanesResult:
    """Routes every lane and writes ``sea_lanes_px.json``."""
    catalogue = json.loads(lanes_file.read_text(encoding="utf-8"))
    positions = json.loads(SETTLEMENTS_PX.read_text(encoding="utf-8"))
    meters_per_px = json.loads(MAP_META.read_text(encoding="utf-8"))["meters_per_px"]
    mask = np.asarray(Image.open(LAND_MASK).convert("L"))
    water = water_cells(mask)
    shore = ndimage.distance_transform_edt(water)
    result = SeaLanesResult(output=output)
    out = []
    for lane in catalogue["lanes"]:
        a, b = positions.get(lane["from"]), positions.get(lane["to"])
        if a is None or b is None:
            result.warnings.append(f"{lane['id']} : port sans position")
            continue
        points = route_lane(water, shore, a, b, lane["kind"])
        if points is None:
            result.warnings.append(f"{lane['id']} : aucun passage par la mer")
            continue
        length_km = (
            float(np.hypot(*np.diff(points, axis=0).T).sum()) * meters_per_px / 1000.0
        )
        out.append(
            {
                "id": lane["id"],
                "from": lane["from"],
                "to": lane["to"],
                "points": [[round(float(x), 1), round(float(y), 1)] for x, y in points],
                "length_km": round(length_km, 1),
            }
        )
        result.total_km += length_km
    result.lanes = len(out)
    document = {
        "description": (
            "Généré par `cent-ans geo sea-lanes` (lot SL1, ADR 0139) depuis "
            "data/naval/sea_lanes.json : tracé des routes maritimes en pixels carte, "
            "routé sur l'eau de land_mask.png, et longueur en kilomètres (coût de la route)."
        ),
        "lanes": out,
    }
    output.write_text(
        json.dumps(document, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    return result
