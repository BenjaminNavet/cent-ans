"""Landmark cities at 1:1 (ADR 0078, lots VH0/VH4): ``data/landmarks_v2/<id>.json``.

A v2 landmark city is georeferenced in EPSG:3035: ``origin_3035`` plus offsets ``[dE, dN]`` in
metres along the grid axes. Most of the file is authored by hand (walls, monuments, districts);
this module regenerates the two derived sections:

* ``streets`` with ``origin: "osm"``: current streets inherited from the medieval layout, read
  from an OpenStreetMap Overpass extract (ODbL, credited in the file's ``sources``) following the
  file's ``osm_streets`` recipe (highway types, excluded later cuts, main streets), clipped to the
  city's districts and simplified;
* ``waters`` with ``origin: "rivers_fine"``: the main river exactly as the fine map draws it
  (``hydro_fine`` tiles), so quays and parcels meet the displayed water.

Hand-made streets and waters (``origin: "hand"``) are kept. Rendering data only, no game rule.
"""

from __future__ import annotations

import json
import math
import re
import time
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from pyproj import Transformer
from shapely.geometry import LineString, Point, Polygon
from shapely.ops import unary_union

REPO = Path(__file__).resolve().parents[3]
DATA_DIR = REPO / "data"
V2_DIR = DATA_DIR / "landmarks_v2"
MAP_DIR = DATA_DIR / "map"
RAW_OSM_DIR = REPO / "tools" / "geo" / "raw" / "osm"
OVERPASS_URL = "https://overpass-api.de/api/interpreter"
USER_AGENT = "cent-ans-geo/1.0 (landmark cities, ODbL credited)"

_to_3035 = Transformer.from_crs("EPSG:4326", "EPSG:3035", always_xy=True)
_to_4326 = Transformer.from_crs("EPSG:3035", "EPSG:4326", always_xy=True)


def load_city(path: Path) -> dict:
    """Read one v2 landmark file."""
    return json.loads(path.read_text(encoding="utf-8"))


def lonlat_to_local(
    lon: np.ndarray | float, lat: np.ndarray | float, origin: list[float]
) -> np.ndarray:
    """WGS84 degrees to local offsets ``[dE, dN]`` (m) from ``origin`` (EPSG:3035)."""
    e, n = _to_3035.transform(np.asarray(lon), np.asarray(lat))
    return np.column_stack([np.atleast_1d(e) - origin[0], np.atleast_1d(n) - origin[1]])


def local_to_lonlat(points: np.ndarray, origin: list[float]) -> np.ndarray:
    """Inverse of :func:`lonlat_to_local`."""
    points = np.asarray(points, dtype=np.float64).reshape(-1, 2)
    lon, lat = _to_4326.transform(points[:, 0] + origin[0], points[:, 1] + origin[1])
    return np.column_stack([lon, lat])


def local_to_units(
    points: np.ndarray, origin: list[float], bounds: list[float]
) -> np.ndarray:
    """Local offsets to map world units (4096 px grid, +y southwards)."""
    mpp = (bounds[2] - bounds[0]) / 4096.0
    points = np.asarray(points, dtype=np.float64).reshape(-1, 2)
    return np.column_stack(
        [
            (points[:, 0] + origin[0] - bounds[0]) / mpp,
            (bounds[3] - (points[:, 1] + origin[1])) / mpp,
        ]
    )


# --- OpenStreetMap ---------------------------------------------------------------------------


def osm_cache_path(city_id: str, raw_dir: Path = RAW_OSM_DIR) -> Path:
    """Cached Overpass answer (not versioned, under ``tools/geo/raw``)."""
    return raw_dir / f"{city_id}_highways.json"


def fetch_osm(city: dict, raw_dir: Path = RAW_OSM_DIR, refresh: bool = False) -> dict:
    """Overpass extract of the highways in the recipe's bounding box (cached)."""
    path = osm_cache_path(city["id"], raw_dir)
    if path.exists() and not refresh:
        return json.loads(path.read_text(encoding="utf-8"))
    import httpx

    lon0, lat0, lon1, lat1 = city["osm_streets"]["bbox_lonlat"]
    query = (
        f'[out:json][timeout:90];way["highway"]({lat0},{lon0},{lat1},{lon1});out geom;'
    )
    response = httpx.post(
        OVERPASS_URL,
        data={"data": query},
        headers={"User-Agent": USER_AGENT},
        timeout=120.0,
    )
    response.raise_for_status()
    raw_dir.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".part")
    tmp.write_text(response.text, encoding="utf-8")
    tmp.replace(path)
    return json.loads(response.text)


def district_area(city: dict, zone: str | None = None) -> Polygon:
    """Union of the city's district polygons (optionally one zone only)."""
    polys = [
        Polygon(d["polygon"])
        for d in city.get("districts", [])
        if zone is None or d["zone"] == zone
    ]
    return unary_union([p.buffer(0) for p in polys]) if polys else Polygon()


def _slug(name: str) -> str:
    out = []
    for ch in name.lower():
        if ch.isalnum() and ch.isascii():
            out.append(ch)
        elif ch in "éèêë":
            out.append("e")
        elif ch in "àâä":
            out.append("a")
        elif ch in "îï":
            out.append("i")
        elif ch in "ôö":
            out.append("o")
        elif ch in "ùûü":
            out.append("u")
        elif ch == "ç":
            out.append("c")
        else:
            out.append("_")
    return "_".join(p for p in "".join(out).split("_") if p)


# Named alleys and courts (footways, service ways) kept as lanes when the recipe sets ``alleys``
# (VH6: the City of London keeps its medieval alleys as footways).
ALLEY_NAME = re.compile(r"\b(Alley|Court|Passage|Yard|Churchyard|Row)$")
ALLEY_HIGHWAYS = ("footway", "service", "steps", "pedestrian")


def osm_streets(city: dict, osm: dict) -> list[dict]:
    """Streets of the recipe, clipped to the districts, as v2 ``streets`` entries."""
    recipe = city["osm_streets"]
    highways = set(recipe["highways"])
    excluded = set(recipe["exclude"])
    main = set(recipe["main"])
    lanes = set(recipe.get("lanes", []))
    keep_unnamed = bool(recipe.get("unnamed", False))
    tolerance = float(recipe.get("simplify_m", 1.5))
    widths = city.get("plan", {}).get("street_width_m", {})
    area = district_area(city).buffer(25.0)
    origin = city["origin_3035"]
    streets = []
    for element in osm.get("elements", []):
        if element.get("type") != "way":
            continue
        tags = element.get("tags", {})
        alley = (
            bool(recipe.get("alleys", False))
            and tags.get("highway") in ALLEY_HIGHWAYS
            and bool(ALLEY_NAME.search(tags.get("name", "")))
        )
        if (tags.get("highway") not in highways and not alley) or tags.get(
            "area"
        ) == "yes":
            continue
        if (
            tags.get("tunnel") in ("yes", "building_passage")
            or tags.get("bridge") == "yes"
        ):
            continue
        name = tags.get("name", "")
        if name in excluded or (not name and not keep_unnamed):
            continue
        geometry = [p for p in element.get("geometry", []) if p]
        if len(geometry) < 2:
            continue
        local = lonlat_to_local(
            [p["lon"] for p in geometry], [p["lat"] for p in geometry], origin
        )
        clipped = LineString(local).intersection(area)
        parts = getattr(clipped, "geoms", [clipped])
        rank = (
            "main"
            if name in main
            else ("lane" if (name in lanes or not name or alley) else "secondary")
        )
        width = float(
            widths.get(rank, {"main": 8.0, "secondary": 5.0, "lane": 3.0}[rank])
        )
        for k, part in enumerate(parts):
            if part.is_empty or part.geom_type != "LineString" or part.length < 12.0:
                continue
            coords = np.asarray(part.simplify(tolerance).coords)
            streets.append(
                {
                    "id": f"osm_{element['id']}" + (f"_{k}" if len(parts) > 1 else ""),
                    "name": name or "venelle",
                    "rank": rank,
                    "width_m": width,
                    "origin": "osm",
                    "points": [
                        [round(float(x), 1), round(float(y), 1)] for x, y in coords
                    ],
                }
            )
    streets.sort(
        key=lambda s: (
            {"main": 0, "secondary": 1, "lane": 2}[s["rank"]],
            s["name"],
            s["id"],
        )
    )
    return streets


# --- Fine rivers -----------------------------------------------------------------------------


def fine_river_lines(
    city: dict, map_dir: Path = MAP_DIR, names: tuple[str, ...] = ("Seine",)
) -> list[dict]:
    """Polylines of the named fine rivers within ``extent_m`` of the origin (local metres)."""
    from cent_ans_tools.geo import fine_tiles, hydro_fine, pyramid

    tiles_dir = map_dir / pyramid.PYRAMID_DIR_NAME / hydro_fine.TILES_SUBDIR
    features_path = tiles_dir / hydro_fine.FEATURES_FILE
    if not features_path.exists():
        return []
    features = json.loads(features_path.read_text(encoding="utf-8"))["features"]
    targets = {hydro_fine.normalise_name(n) for n in names}
    wanted = {
        i
        for i, f in enumerate(features)
        if hydro_fine.normalise_name(f["name"]) in targets
    }
    from cent_ans_tools.geo import pyramid  # noqa: PLC0415 - heavy, import cycle

    bounds = pyramid.map_bounds(map_dir)  # frame of the fine tiles (OM2)
    mpp = (bounds[2] - bounds[0]) / 4096.0
    origin = city["origin_3035"]
    reach = float(city["extent_m"]) + 400.0
    center = local_to_units(np.zeros((1, 2)), origin, bounds)[0]
    side = fine_tiles.tile_units(fine_tiles.TILE_LEVEL)
    r_units = reach / mpp
    lines = []
    for col in range(
        int((center[0] - r_units) // side), int((center[0] + r_units) // side) + 1
    ):
        for row in range(
            int((center[1] - r_units) // side), int((center[1] + r_units) // side) + 1
        ):
            path = tiles_dir / f"E{fine_tiles.TILE_LEVEL}" / f"{col}_{row}.bin"
            if not path.exists():
                continue
            for line in fine_tiles.decode(path.read_bytes())["lines"]:
                if line["feature"] not in wanted:
                    continue
                xy = np.asarray(line["xy"], dtype=np.float64)
                local = np.column_stack(
                    [
                        bounds[0] + xy[:, 0] * mpp - origin[0],
                        bounds[3] - xy[:, 1] * mpp - origin[1],
                    ]
                )
                inside = np.hypot(local[:, 0], local[:, 1]) <= reach
                if inside.sum() < 2:
                    continue
                lines.append(
                    {
                        "name": hydro_fine.normalise_name(
                            features[line["feature"]]["name"]
                        ).title(),
                        "points": local[inside],
                        "width": np.asarray(line["w"], dtype=np.float64)[inside],
                    }
                )
    return lines


def chain_lines(lines: list[dict], join_m: float = 30.0) -> list[dict]:
    """Join tile pieces of the same river end to end (tiles cut the lines)."""
    pieces = [dict(line) for line in lines]
    merged = True
    while merged and len(pieces) > 1:
        merged = False
        for i in range(len(pieces)):
            for j in range(len(pieces)):
                if i == j or pieces[i]["name"] != pieces[j]["name"]:
                    continue
                a, b = pieces[i], pieces[j]
                if np.hypot(*(a["points"][-1] - b["points"][0])) <= join_m:
                    a["points"] = np.vstack([a["points"], b["points"][1:]])
                    a["width"] = np.concatenate([a["width"], b["width"][1:]])
                    pieces.pop(j)
                    merged = True
                    break
            if merged:
                break
    return pieces


def fine_waters(
    city: dict, map_dir: Path = MAP_DIR, step_m: float = 40.0
) -> list[dict]:
    """``waters`` entries (origin ``rivers_fine``) for the rivers the fine map draws."""
    names = tuple(city.get("fine_rivers", ["Seine"]))
    out = []
    for k, line in enumerate(chain_lines(fine_river_lines(city, map_dir, names))):
        pts = line["points"]
        if len(pts) < 2:
            continue
        along = np.concatenate([[0.0], np.cumsum(np.hypot(*np.diff(pts, axis=0).T))])
        if along[-1] < step_m:
            continue
        samples = np.arange(0.0, along[-1] + 1e-6, step_m)
        if samples[-1] < along[-1]:
            samples = np.append(samples, along[-1])
        x = np.interp(samples, along, pts[:, 0])
        y = np.interp(samples, along, pts[:, 1])
        w = np.interp(samples, along, line["width"])
        out.append(
            {
                "id": f"{_slug(line['name'])}_{k}",
                "name": line["name"],
                "kind": "river",
                "origin": "rivers_fine",
                "draw": False,
                "points": [
                    [round(float(a), 1), round(float(b), 1), round(float(c), 1)]
                    for a, b, c in zip(x, y, w, strict=True)
                ],
            }
        )
    return out


# --- Build ------------------------------------------------------------------------------------


@dataclass
class LandmarksResult:
    """Summary of a ``cent-ans geo landmarks`` run."""

    cities: list[str]
    streets: int
    waters: int
    seconds: float

    def summary(self) -> str:
        """One-line French summary."""
        return (
            f"villes 1:1 : {', '.join(self.cities) or 'aucune'} ; {self.streets} rues générées (OSM, ALPAGE), "
            f"{self.waters} cours d'eau fins ({self.seconds:.1f} s)"
        )


_NUMBER_ARRAY = re.compile(r"\[\s*(-?[\d.eE+-]+(?:,\s*-?[\d.eE+-]+)*)\s*\]")


def dumps_city(city: dict) -> str:
    """Indented JSON, each array of numbers (a point) kept on one line."""
    text = json.dumps(city, ensure_ascii=False, indent=1)
    return _NUMBER_ARRAY.sub(
        lambda m: "[" + ", ".join(v.strip() for v in m.group(1).split(",")) + "]", text
    )


def write_city(path: Path, city: dict) -> None:
    """Write a v2 file (indented, stable key order as authored)."""
    tmp = path.with_suffix(".part")
    tmp.write_text(dumps_city(city) + "\n", encoding="utf-8")
    tmp.replace(path)


def build(
    only: list[str] | None = None,
    refresh_osm: bool = False,
    v2_dir: Path = V2_DIR,
    map_dir: Path = MAP_DIR,
    raw_dir: Path = RAW_OSM_DIR,
    log=print,  # noqa: ANN001
) -> LandmarksResult:
    """Regenerate the derived sections of every (or the listed) v2 landmark files."""
    started = time.time()
    done, n_streets, n_waters = [], 0, 0
    for path in sorted(v2_dir.glob("*.json")):
        city = load_city(path)
        if only and city["id"] not in only:
            continue
        if "osm_streets" in city:
            osm = fetch_osm(city, raw_dir, refresh_osm)
            generated = osm_streets(city, osm)
            city["streets"] = [
                s for s in city["streets"] if s["origin"] != "osm"
            ] + generated
            n_streets += len(generated)
        if "alpage" in city:
            # Paris (VH5): streets of 1380 and Vasserot parcels from ALPAGE (ODbL).
            from cent_ans_tools.geo import alpage

            generated = alpage.alpage_streets(city)
            city["streets"] = [
                s for s in city["streets"] if s["origin"] != "alpage"
            ] + generated
            n_streets += len(generated)
            if "parcels" in city["alpage"]:
                city["parcels"] = alpage.alpage_parcels(city, city["streets"])
                log(f"{city['id']} : {len(city['parcels'])} parcelles ALPAGE")
        # ``fine_rivers: []`` (Paris): the fine river is not used, its section is dropped.
        waters = (
            fine_waters(city, map_dir) if city.get("fine_rivers", ["Seine"]) else []
        )
        if not city.get("fine_rivers", ["Seine"]):
            city["waters"] = [
                w for w in city.get("waters", []) if w["origin"] != "rivers_fine"
            ]
        elif waters:
            city["waters"] = [
                w for w in city.get("waters", []) if w["origin"] != "rivers_fine"
            ] + waters
            n_waters += len(waters)
        else:
            log(
                f"{city['id']} : aucun fleuve fin trouvé (pyramide absente ?), section gardée"
            )
        write_city(path, city)
        done.append(city["id"])
        log(
            f"{city['id']} : {len(city['streets'])} rues, {len(city.get('waters', []))} eaux"
        )
    return LandmarksResult(done, n_streets, n_waters, time.time() - started)


def wall_length(city: dict) -> float:
    """Total length (m) of the walls (tests, docs)."""
    total = 0.0
    for wall in city.get("walls", []):
        pts = wall["points"] + ([wall["points"][0]] if wall.get("closed") else [])
        total += sum(math.dist(a, b) for a, b in zip(pts, pts[1:], strict=False))
    return total


def point_in_districts(city: dict, point: list[float]) -> bool:
    """True if ``point`` lies in one of the districts."""
    return district_area(city).contains(Point(point))
