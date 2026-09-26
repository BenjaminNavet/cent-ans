"""ALPAGE GIS data for Paris at 1:1 (lot VH5, ADR 0078).

ALPAGE (AnaLyse diachronique de l'espace urbain PArisien : approche GEomatique, LAMOP / Huma-Num,
dir. H. Noizet) publishes its vector layers under the ODbL 1.0 with free download:
https://alpage.huma-num.fr/gis-data/ . Two sets are used for ``data/landmarks_v2/paris.json``:

* **Paris in 1380** (P. Rouet): the medieval street network (``1380_voies``), blocks, land use;
  the streets replace the OpenStreetMap recipe for Paris (``origin: "alpage"``);
* **Vasserot data version 1** (A.-L. Bethe): the parcels of the Vasserot atlas (1810-1836). They
  post-date 1340 by five centuries, so they are only used as a template of the medieval strips:
  a parcel is kept when it fronts a 1380 street, is not crossed by one, does not overlap a
  non-residential 1380 land use (church, convent, college, market, royal building, water…) and has
  a plausible size. Each kept parcel becomes ``[dE, dN, angle_deg, front_m, depth_m]``: midpoint
  of its street frontage, direction of its inward normal (v2 convention), frontage and depth.

The raw GeoPackages (EPSG:2154) are cached under ``tools/geo/raw/alpage/`` (not versioned).
Rendering data only, no game rule.
"""

from __future__ import annotations

import io
import math
import sqlite3
import zipfile
from pathlib import Path

import numpy as np
from pyproj import Transformer
from shapely import wkb
from shapely.geometry import LineString, MultiLineString, Polygon
from shapely.ops import transform, unary_union
from shapely.strtree import STRtree

REPO = Path(__file__).resolve().parents[3]
RAW_ALPAGE_DIR = REPO / "tools" / "geo" / "raw" / "alpage"
BASE_URL = "https://alpage.huma-num.fr/wp-content/uploads/2026/09/"
LAYERS = (
    "1380_voies",
    "1380_ilots",
    "1380_usages_sol",
    "1380_hydro",
    "Vasserot_1_Parcelles",
)
USER_AGENT = "cent-ans-geo/1.0 (landmark cities, ODbL credited)"
CREDIT = (
    "© ALPAGE : P. Rouet (Paris en 1380), A.-L. Bethe (données Vasserot v1) — ODbL 1.0, "
    "https://alpage.huma-num.fr/gis-data/"
)

_to_3035 = Transformer.from_crs("EPSG:2154", "EPSG:3035", always_xy=True)


# --- Raw data ---------------------------------------------------------------------------------


def layer_path(name: str, raw_dir: Path = RAW_ALPAGE_DIR) -> Path:
    """GeoPackage of layer ``name`` in the cache."""
    return raw_dir / name / name / f"{name}.gpkg"


def fetch_layer(name: str, raw_dir: Path = RAW_ALPAGE_DIR) -> Path:
    """Download and unpack one ALPAGE layer (cached)."""
    path = layer_path(name, raw_dir)
    if path.exists():
        return path
    import httpx

    response = httpx.get(
        BASE_URL + f"{name}.zip",
        headers={"User-Agent": USER_AGENT},
        timeout=180.0,
        follow_redirects=True,
    )
    response.raise_for_status()
    target = raw_dir / name
    target.mkdir(parents=True, exist_ok=True)
    zipfile.ZipFile(io.BytesIO(response.content)).extractall(target)
    return path


def _gpkg_geometry(blob: bytes):  # noqa: ANN202
    """Shapely geometry of a GeoPackage binary blob (header + WKB)."""
    envelope = (blob[3] >> 1) & 7
    skip = {0: 0, 1: 32, 2: 48, 3: 48, 4: 64}[envelope]
    return wkb.loads(bytes(blob[8 + skip :]))


def read_layer(name: str, raw_dir: Path = RAW_ALPAGE_DIR) -> list[dict]:
    """Features of a layer: attribute dict plus ``geom`` (EPSG:2154)."""
    path = fetch_layer(name, raw_dir)
    con = sqlite3.connect(path)
    try:
        table = con.execute("select table_name from gpkg_contents").fetchone()[0]
        cols = [r[1] for r in con.execute(f'pragma table_info("{table}")')]
        out = []
        for row in con.execute(f'select * from "{table}"'):
            item = dict(zip(cols, row, strict=True))
            blob = item.pop("geom")
            if blob is None:
                continue
            item["geom"] = _gpkg_geometry(blob)
            out.append(item)
        return out
    finally:
        con.close()


def to_local(geom, origin: list[float]):  # noqa: ANN001, ANN201
    """EPSG:2154 geometry to local offsets ``[dE, dN]`` (m) from ``origin`` (EPSG:3035)."""

    def project(x, y, z=None):  # noqa: ANN001, ANN202, ARG001
        e, n = _to_3035.transform(x, y)
        return (np.asarray(e) - origin[0], np.asarray(n) - origin[1])

    return transform(project, geom)


# --- Streets ----------------------------------------------------------------------------------


def _district_area(city: dict, ids: list[str] | None = None) -> Polygon:
    polys = [
        Polygon(d["polygon"]).buffer(0)
        for d in city.get("districts", [])
        if ids is None or d["id"] in ids
    ]
    return unary_union(polys) if polys else Polygon()


def _lines(geom) -> list[LineString]:  # noqa: ANN001
    if geom.is_empty:
        return []
    if isinstance(geom, LineString):
        return [geom]
    if isinstance(geom, MultiLineString):
        return list(geom.geoms)
    return [g for g in getattr(geom, "geoms", []) if isinstance(g, LineString)]


def alpage_streets(city: dict, raw_dir: Path = RAW_ALPAGE_DIR) -> list[dict]:
    """Streets of Paris in 1380 (ALPAGE, P. Rouet), clipped to the districts (+25 m)."""
    recipe = city["alpage"]["streets"]
    origin = city["origin_3035"]
    widths = city.get("plan", {}).get("street_width_m", {})
    skip_regions = set(recipe.get("skip_regions", []))
    excluded = set(recipe.get("exclude", []))
    excluded_ids = set(recipe.get("exclude_ids", []))
    main_extra = set(recipe.get("main", []))
    lane_words = tuple(recipe.get("lane_words", ["Ruelle", "Cul", "Impasse", "Passage"]))
    tolerance = float(recipe.get("simplify_m", 1.0))
    area = _district_area(city).buffer(25.0)
    streets = []
    for item in read_layer(recipe.get("layer", "1380_voies"), raw_dir):
        if item.get("REGION") in skip_regions:
            continue
        name = (item.get("NOM") or "").strip()
        sid = f"alpage_{item['fid']}"
        if name in excluded or sid in excluded_ids:
            continue
        rank = "secondary"
        if item.get("AXE_MAJEUR") == "OUI" or name in main_extra:
            rank = "main"
        elif not name or name.startswith(lane_words):
            rank = "lane"
        width = float(widths.get(rank, {"main": 8.0, "secondary": 5.0, "lane": 3.0}[rank]))
        local = to_local(item["geom"], origin)
        parts = [p for line in _lines(local) for p in _lines(line.intersection(area))]
        for k, part in enumerate(parts):
            if part.length < 8.0:
                continue
            coords = np.asarray(part.simplify(tolerance).coords)
            streets.append(
                {
                    "id": sid + (f"_{k}" if len(parts) > 1 else ""),
                    "name": name or "venelle",
                    "rank": rank,
                    "width_m": width,
                    "origin": "alpage",
                    "points": [[round(float(x), 1), round(float(y), 1)] for x, y in coords],
                }
            )
    streets.sort(key=lambda s: ({"main": 0, "secondary": 1, "lane": 2}[s["rank"]], s["name"], s["id"]))
    return streets


# --- Parcels ----------------------------------------------------------------------------------


def _frontage(poly: Polygon, street_tree: STRtree, street_lines: list, street_half: list, reach: float):  # noqa: ANN001, ANN202
    """Best street frontage of a parcel: (midpoint, inward normal, length, depth) or None."""
    ring = np.asarray(poly.exterior.coords)[:-1]
    n = len(ring)
    if n < 3:
        return None
    centroid = np.asarray(poly.centroid.coords[0])
    best = None
    for i in range(n):
        a = ring[i]
        b = ring[(i + 1) % n]
        seg = b - a
        length = float(np.hypot(*seg))
        if length < 2.5:
            continue
        mid = (a + b) * 0.5
        u = seg / length
        near = street_tree.query(LineString([a, b]).buffer(reach))
        ok = False
        for j in near:
            line = street_lines[j]
            d = line.distance(LineString([a, b]).interpolate(0.5, normalized=True))
            if d > street_half[j] + reach:
                continue
            # Parallel to the street at the projection point.
            s = line.project(LineString([a, b]).interpolate(0.5, normalized=True))
            p0 = np.asarray(line.interpolate(max(s - 2.0, 0.0)).coords[0])
            p1 = np.asarray(line.interpolate(min(s + 2.0, line.length)).coords[0])
            t = p1 - p0
            tl = float(np.hypot(*t))
            if tl < 1e-6:
                continue
            if abs(float(np.dot(u, t / tl))) > math.cos(math.radians(30.0)):
                ok = True
                break
        if not ok:
            continue
        normal = np.array([-u[1], u[0]])
        if np.dot(centroid - mid, normal) < 0.0:
            normal = -normal
        depth = float(np.max((ring - mid) @ normal))
        if best is None or length > best[2]:
            best = (mid, normal, length, depth)
    return best


def alpage_parcels(city: dict, streets: list[dict], raw_dir: Path = RAW_ALPAGE_DIR) -> list[list[float]]:
    """Vasserot parcels kept as 1340 strips: ``[dE, dN, angle_deg, front_m, depth_m]``."""
    recipe = city["alpage"]["parcels"]
    origin = city["origin_3035"]
    zone = _district_area(city, recipe.get("districts"))
    min_area = float(recipe.get("min_area_m2", 12.0))
    max_area = float(recipe.get("max_area_m2", 2500.0))
    max_front = float(recipe.get("max_front_m", 30.0))
    max_depth = float(recipe.get("max_depth_m", 60.0))
    reach = float(recipe.get("reach_m", 4.0))
    overlap = float(recipe.get("max_landuse_overlap", 0.3))
    blocked_legends = set(recipe.get("exclude_landuse", []))
    landuse = []
    if blocked_legends:
        for item in read_layer("1380_usages_sol", raw_dir):
            if item.get("LEGENDE") in blocked_legends:
                landuse.append(to_local(item["geom"], origin).buffer(0))
    land_tree = STRtree(landuse) if landuse else None
    live = [s for s in streets if s["origin"] in ("alpage", "osm", "hand")]
    street_lines = [LineString(s["points"]) for s in live]
    street_half = [float(s.get("width_m", 5.0)) * 0.5 for s in live]
    street_tree = STRtree(street_lines)
    out = []
    for item in read_layer(recipe.get("layer", "Vasserot_1_Parcelles"), raw_dir):
        geom = to_local(item["geom"], origin).buffer(0)
        if geom.is_empty:
            continue
        poly = max(getattr(geom, "geoms", [geom]), key=lambda g: g.area)
        if not (min_area <= poly.area <= max_area):
            continue
        if not zone.contains(poly.representative_point()):
            continue
        # A street of 1380 across the parcel: later subdivision, not a medieval strip.
        crossing = street_tree.query(poly)
        if any(poly.buffer(-1.5).intersects(street_lines[j]) for j in crossing):
            continue
        if land_tree is not None:
            shared = sum(
                poly.intersection(landuse[j]).area for j in land_tree.query(poly)
            )
            if shared > overlap * poly.area:
                continue
        simple = poly.simplify(0.4)
        if not isinstance(simple, Polygon) or simple.is_empty:
            continue
        front = _frontage(simple, street_tree, street_lines, street_half, reach)
        if front is None:
            continue
        mid, normal, length, depth = front
        if depth < 4.0:
            continue
        angle = math.degrees(math.atan2(normal[1], normal[0]))
        out.append(
            [
                round(float(mid[0]), 1),
                round(float(mid[1]), 1),
                round(angle, 1),
                round(min(length, max_front), 1),
                round(min(depth, max_depth), 1),
            ]
        )
    out.sort(key=lambda p: (p[1], p[0]))
    return out
