"""Erase modern earthworks from 1-5 m DTMs (lot ZG3, ADR 0036).

A DTM of the 2020s shows the embankments and cuttings of motorways and
railways, quarries, landfills, reservoirs, modern canals and runways. Before a
detail zone is baked, those features are taken from OpenStreetMap (Overpass
API, © OpenStreetMap contributors, ODbL 1.0; the answers stay in the gitignored
raw cache, nothing is redistributed), buffered by a width per feature class,
rasterised, dilated, and the heights under the mask are replaced by the
harmonic (Laplace) interpolation of the surrounding ground. Everything else
(terraces, banks, mottes, ancient ditches) is kept.

Tunnels and bridges are left out (they do not touch the ground; filling under
a bridge would fill the river). Canals are left out in city zones, where they
are often medieval (Bruges, Sluis).
"""

from __future__ import annotations

import json
import time
from collections.abc import Iterable
from pathlib import Path

import httpx
import numpy as np
import shapely
from pyproj import Transformer
from rasterio import features
from rasterio.transform import Affine
from scipy import ndimage, sparse
from scipy.sparse.linalg import spsolve

from cent_ans_tools.geo.detail_sources import DETAIL_RAW_DIR, USER_AGENT

OVERPASS_URL = "https://overpass-api.de/api/interpreter"
OVERPASS_ATTEMPTS = 6
OSM_FILE = "osm_modern.json"

#: Half-width (m) of the erased band around each feature class (linear) or
#: margin around its area (polygons).
BUFFER_M: dict[str, float] = {
    "motorway": 32.0,
    "trunk": 26.0,
    "motorway_link": 18.0,
    "trunk_link": 16.0,
    "rail": 20.0,
    "light_rail": 12.0,
    "narrow_gauge": 10.0,
    "disused": 14.0,
    "abandoned": 12.0,
    "canal": 22.0,
    "runway": 60.0,
    "taxiway": 25.0,
    "area": 12.0,
}
#: Extra dilation of the rasterised mask, in pixels.
DILATE_PX = 2


def overpass_query(bbox_lonlat: tuple[float, float, float, float], canals: bool) -> str:
    """Overpass QL for the modern features inside ``(lon_min, lat_min, lon_max, lat_max)``."""
    lon_min, lat_min, lon_max, lat_max = bbox_lonlat
    box = f"{lat_min:.5f},{lon_min:.5f},{lat_max:.5f},{lon_max:.5f}"
    parts = [
        f'way["highway"~"^(motorway|trunk|motorway_link|trunk_link)$"]({box});',
        f'way["railway"~"^(rail|light_rail|narrow_gauge|disused|abandoned)$"]({box});',
        f'way["landuse"~"^(quarry|landfill|reservoir|basin)$"]({box});',
        f'relation["landuse"~"^(quarry|landfill|reservoir|basin)$"]({box});',
        f'way["natural"="water"]["water"~"^(reservoir|basin)$"]({box});',
        f'relation["natural"="water"]["water"~"^(reservoir|basin)$"]({box});',
        f'way["aeroway"~"^(runway|taxiway)$"]({box});',
    ]
    if canals:
        parts.append(f'way["waterway"="canal"]({box});')
    return "[out:json][timeout:600];(" + "".join(parts) + ");out tags geom;"


def fetch_osm(
    zone_id: str,
    bbox_lonlat: tuple[float, float, float, float],
    canals: bool,
    force: bool = False,
) -> Path:
    """Overpass answer for a zone, cached in ``raw/detail/<zone>/osm_modern.json``."""
    path = DETAIL_RAW_DIR / zone_id / OSM_FILE
    if path.exists() and not force:
        return path
    path.parent.mkdir(parents=True, exist_ok=True)
    query = overpass_query(bbox_lonlat, canals)
    delay = 5.0
    for attempt in range(OVERPASS_ATTEMPTS):
        try:
            response = httpx.post(
                OVERPASS_URL,
                data={"data": query},
                headers={"User-Agent": USER_AGENT},
                timeout=700.0,
            )
            if response.status_code == 200:
                payload = response.json()
                partial = path.with_suffix(".part")
                partial.write_text(json.dumps(payload), encoding="utf-8")
                partial.replace(path)
                time.sleep(2.0)  # politeness between zones
                return path
            error = f"HTTP {response.status_code}"
        except (httpx.HTTPError, ValueError) as exc:
            error = str(exc)
        if attempt == OVERPASS_ATTEMPTS - 1:
            raise RuntimeError(f"Overpass {zone_id}: {error}")
        time.sleep(delay)
        delay *= 2.0
    return path


def feature_class(tags: dict[str, str]) -> str | None:
    """Buffer class of an OSM element (``None`` when it must be kept)."""
    if tags.get("tunnel") not in (None, "no") or tags.get("bridge") not in (None, "no"):
        return None
    if tags.get("covered") == "yes" or tags.get("location") == "underground":
        return None
    highway = tags.get("highway")
    if highway in ("motorway", "trunk", "motorway_link", "trunk_link"):
        return highway
    railway = tags.get("railway")
    if railway in ("rail", "light_rail", "narrow_gauge", "disused", "abandoned"):
        return railway
    if tags.get("aeroway") in ("runway", "taxiway"):
        return tags["aeroway"]
    if tags.get("waterway") == "canal":
        return "canal"
    if tags.get("landuse") in ("quarry", "landfill", "reservoir", "basin"):
        return "area"
    if tags.get("natural") == "water" and tags.get("water") in ("reservoir", "basin"):
        return "area"
    return None


def _way_coords(geometry: list[dict]) -> list[tuple[float, float]]:
    return [(point["lon"], point["lat"]) for point in geometry if point]


def osm_shapes(payload: dict) -> list[tuple[shapely.Geometry, str]]:
    """``(geometry in lon/lat, class)`` for every relevant element of an Overpass answer."""
    shapes: list[tuple[shapely.Geometry, str]] = []
    for element in payload.get("elements", []):
        tags = element.get("tags", {})
        kind = feature_class(tags)
        if kind is None:
            continue
        if element["type"] == "way" and "geometry" in element:
            coords = _way_coords(element["geometry"])
            if len(coords) < 2:
                continue
            closed = coords[0] == coords[-1] and len(coords) >= 4
            if kind == "area" and closed:
                shapes.append((shapely.Polygon(coords), kind))
            else:
                shapes.append((shapely.LineString(coords), kind))
        elif element["type"] == "relation":
            rings = [
                shapely.Polygon(_way_coords(member["geometry"]))
                for member in element.get("members", [])
                if member.get("role") == "outer"
                and "geometry" in member
                and len(member["geometry"]) >= 4
                and member["geometry"][0] == member["geometry"][-1]
            ]
            shapes.extend((ring, kind) for ring in rings if ring.is_valid)
    return shapes


def modern_mask(
    shapes: Iterable[tuple[shapely.Geometry, str]],
    transform: Affine,
    shape: tuple[int, int],
    crs: str = "EPSG:3035",
    dilate_px: int = DILATE_PX,
) -> np.ndarray:
    """Boolean mask of the buffered modern features on a raster grid."""
    to_map = Transformer.from_crs("EPSG:4326", crs, always_xy=True)
    buffered = []
    for geometry, kind in shapes:
        projected = shapely.transform(
            geometry, lambda xy: np.column_stack(to_map.transform(xy[:, 0], xy[:, 1]))
        )
        buffered.append(shapely.buffer(projected, BUFFER_M[kind]))
    mask = np.zeros(shape, dtype=bool)
    if buffered:
        burned = features.rasterize(
            ((geometry, 1) for geometry in buffered if not geometry.is_empty),
            out_shape=shape,
            transform=transform,
            fill=0,
            dtype=np.uint8,
            all_touched=True,
        )
        mask = burned.astype(bool)
    if dilate_px > 0 and mask.any():
        mask = ndimage.binary_dilation(mask, iterations=dilate_px)
    return mask


def laplace_fill(height: np.ndarray, mask: np.ndarray) -> np.ndarray:
    """Replace ``height`` under ``mask`` by the harmonic interpolation of its borders.

    Each connected component of ``mask`` is solved on its own (direct sparse
    solve of the 5-point Laplace equation with Dirichlet values taken from the
    known neighbours). Neighbours outside the raster or NaN act as free
    (Neumann) borders; a component without any known neighbour stays as is.
    """
    result = height.astype(np.float32, copy=True)
    labels, count = ndimage.label(mask)
    if count == 0:
        return result
    rows_total, cols_total = result.shape
    for index, window in enumerate(ndimage.find_objects(labels), start=1):
        r0 = max(window[0].start - 1, 0)
        r1 = min(window[0].stop + 1, rows_total)
        c0 = max(window[1].start - 1, 0)
        c1 = min(window[1].stop + 1, cols_total)
        component = labels[r0:r1, c0:c1] == index
        known = result[r0:r1, c0:c1]
        solved = _solve_component(known, component)
        if solved is not None:
            block = result[r0:r1, c0:c1]
            block[component] = solved
    return result


def _solve_component(values: np.ndarray, component: np.ndarray) -> np.ndarray | None:
    """Harmonic values on ``component`` given the finite ``values`` around it."""
    rows, cols = component.shape
    ids = -np.ones(component.shape, dtype=np.int64)
    ids[component] = np.arange(int(component.sum()))
    n = int(component.sum())
    rr, cc = np.nonzero(component)
    diag = np.zeros(n, dtype=np.float64)
    rhs = np.zeros(n, dtype=np.float64)
    off_rows: list[np.ndarray] = []
    off_cols: list[np.ndarray] = []
    anchored = False
    for dr, dc in ((-1, 0), (1, 0), (0, -1), (0, 1)):
        nr, nc = rr + dr, cc + dc
        inside = (nr >= 0) & (nr < rows) & (nc >= 0) & (nc < cols)
        me = np.nonzero(inside)[0]
        nr, nc = nr[inside], nc[inside]
        neighbour_id = ids[nr, nc]
        is_unknown = neighbour_id >= 0
        neighbour_value = values[nr, nc]
        is_known = ~is_unknown & np.isfinite(neighbour_value)
        diag[me[is_unknown | is_known]] += 1.0
        np.add.at(rhs, me[is_known], neighbour_value[is_known])
        anchored = anchored or bool(is_known.any())
        off_rows.append(me[is_unknown])
        off_cols.append(neighbour_id[is_unknown])
    if not anchored:
        return None
    # Pixels with no usable neighbour at all keep a unit diagonal (isolated).
    diag[diag == 0] = 1.0
    row_index = np.concatenate([np.arange(n), *off_rows])
    col_index = np.concatenate([np.arange(n), *off_cols])
    data = np.concatenate([diag, *[-np.ones(len(r)) for r in off_rows]])
    matrix = sparse.csr_matrix((data, (row_index, col_index)), shape=(n, n))
    solution = spsolve(matrix.tocsc(), rhs)
    return solution.astype(np.float32)


def fill_small_holes(height: np.ndarray, max_px: int) -> np.ndarray:
    """Laplace-fill nodata holes that do not touch the raster edge and are small.

    Rivers, canals and ponds are often nodata in lidar DTMs; the sea and large
    lakes (touching the edge or larger than ``max_px``) stay NaN.
    """
    holes = ~np.isfinite(height)
    if not holes.any():
        return height
    labels, count = ndimage.label(holes)
    sizes = ndimage.sum_labels(holes, labels, np.arange(1, count + 1))
    edge = np.unique(
        np.concatenate([labels[0], labels[-1], labels[:, 0], labels[:, -1]])
    )
    keep = np.zeros(count + 1, dtype=bool)
    keep[1:] = sizes <= max_px
    keep[edge] = False
    small = keep[labels]
    if not small.any():
        return height
    return laplace_fill(height, small)
