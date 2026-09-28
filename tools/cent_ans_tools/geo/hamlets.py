"""Decorative hamlets from GeoNames -> ``data/map/hamlets.json`` (spec 2026-09-24 § 3.3).

Source: GeoNames ``cities500.zip`` (populated places of 500+ inhabitants,
https://download.geonames.org/export/dump/, CC BY 4.0, © GeoNames). Selection:

1. feature class ``P`` with a code of :data:`PLACE_CODES` (``PPL*`` minus
   sections, historical, abandoned and destroyed places), inside a playable
   province of ``province_ids.png``;
2. more than :data:`MIN_SETTLEMENT_DISTANCE_KM` from every settlement;
3. about :data:`TARGET_COUNT` places shared between provinces by half area,
   half 1337 population (``data/provinces``), so dense regions get more;
4. inside a province, candidates in a deterministic shuffled order (the modern
   population would favour industrial towns) accepted greedily with a spacing
   of ``max(MIN_SPACING_KM, SPACING_FACTOR x sqrt(area / quota))`` to spread
   them evenly, then a second pass at :data:`MIN_SPACING_KM` fills the quota
   where the first one fell short. :data:`MIN_SPACING_KM` also holds across
   province borders.

Names are GeoNames ``name`` (local form), not ``asciiname``. No game state:
the renderer burns them according to their province's devastation.
"""

from __future__ import annotations

import json
import time
import zipfile
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from scipy.spatial import cKDTree

from cent_ans_tools.geo import download, settlements

MAP_DIR = settlements.MAP_DIR
HAMLETS_FILE = "hamlets.json"
GEONAMES_URL = "https://download.geonames.org/export/dump/cities500.zip"
GEONAMES_FILE = "cities500.zip"
PLACE_CODES = {
    "PPL",
    "PPLA",
    "PPLA2",
    "PPLA3",
    "PPLA4",
    "PPLA5",
    "PPLC",
    "PPLF",
    "PPLG",
    "PPLL",
    "PPLR",
    "PPLS",
}
TARGET_COUNT = 3000
MIN_SPACING_KM = 6.0
SPACING_FACTOR = 0.6
# DC3 (ADR 0082): 3 -> 5 km, the settlement models (up to ~4 km across) covered 109
# hamlets once the map held 1 192 settlements.
MIN_SETTLEMENT_DISTANCE_KM = 5.0
AREA_SHARE = 0.5
SEED = 1337


@dataclass
class HamletResult:
    """Output of :func:`build`."""

    path: Path
    count: int
    candidates: int
    provinces: int
    seconds: float


def read_geonames(archive: Path) -> dict[str, np.ndarray]:
    """Populated places of ``cities500.txt``: ``id``, ``name``, ``lon``, ``lat``."""
    ids, names, lons, lats = [], [], [], []
    with zipfile.ZipFile(archive) as zipped, zipped.open("cities500.txt") as handle:
        for raw in handle:
            fields = raw.decode("utf-8").rstrip("\n").split("\t")
            if fields[6] != "P" or fields[7] not in PLACE_CODES:
                continue
            ids.append(int(fields[0]))
            names.append(fields[1])
            lats.append(float(fields[4]))
            lons.append(float(fields[5]))
    return {
        "id": np.array(ids, dtype=np.int64),
        "name": np.array(names, dtype=object),
        "lon": np.array(lons),
        "lat": np.array(lats),
    }


def province_quotas(
    area_px: dict[str, float], population: dict[str, float], total: int
) -> dict[str, int]:
    """Hamlets per province, proportional to a blend of area and population shares."""
    area_sum = sum(area_px.values()) or 1.0
    population_sum = sum(population.values()) or 1.0
    shares = {
        pid: AREA_SHARE * area_px[pid] / area_sum
        + (1 - AREA_SHARE) * population.get(pid, 0.0) / population_sum
        for pid in area_px
    }
    return {pid: max(1, int(round(total * share))) for pid, share in shares.items()}


def province_population(province: dict) -> float:
    """Sum of the class counts of a province (0 when unknown)."""
    classes = province.get("population", {}).get("classes", {})
    return float(sum(c.get("count", 0) for c in classes.values()))


def select(
    points: np.ndarray,
    province_of: np.ndarray,
    quotas: dict[str, int],
    area_km2: dict[str, float],
    km_per_px: float,
) -> np.ndarray:
    """Indices of the accepted candidates (see module docstring for the rule)."""
    rng = np.random.default_rng(SEED)
    order = rng.permutation(len(points))
    min_px = MIN_SPACING_KM / km_per_px
    accepted: list[int] = []
    global_points: list[np.ndarray] = []

    def far_from(candidates: list[np.ndarray], point: np.ndarray, limit: float) -> bool:
        if not candidates:
            return True
        stack = np.asarray(candidates)
        return bool(np.min(np.hypot(*(stack - point).T)) >= limit)

    for province in sorted(quotas):
        members = order[province_of[order] == province]
        quota = quotas[province]
        spacing_km = max(
            MIN_SPACING_KM, SPACING_FACTOR * np.sqrt(area_km2[province] / quota)
        )
        chosen: list[int] = []
        chosen_points: list[np.ndarray] = []
        for spacing_px in (spacing_km / km_per_px, min_px):
            for index in members:
                if len(chosen) >= quota:
                    break
                if index in chosen:
                    continue
                point = points[index]
                if far_from(chosen_points, point, spacing_px) and far_from(
                    global_points, point, min_px
                ):
                    chosen.append(int(index))
                    chosen_points.append(point)
        accepted += chosen
        global_points += chosen_points
    return np.array(sorted(accepted), dtype=np.int64)


def build(force: bool = False, map_dir: Path = MAP_DIR) -> HamletResult:
    """Write ``hamlets.json``: ``[{"name", "px": [x, y], "province"}]``."""
    started = time.perf_counter()
    archive = download.download_file(
        GEONAMES_URL, download.RAW_DIR / "geonames" / GEONAMES_FILE, force
    )
    places = read_geonames(archive)
    grid = settlements.provinces_step.load_grid(map_dir)
    labels = settlements.load_labels(map_dir)
    geometry = settlements.load_province_geometry(map_dir)
    province_by_index = {props["index"]: pid for pid, props in geometry.items()}
    km_per_px = grid.meters_per_px / 1000.0

    lon_min, lon_max, lat_min, lat_max = grid.geographic_extent()
    inside = (
        (places["lon"] >= lon_min)
        & (places["lon"] <= lon_max)
        & (places["lat"] >= lat_min)
        & (places["lat"] <= lat_max)
    )
    places = {key: value[inside] for key, value in places.items()}
    xs, ys = grid.lonlat_to_pixel(places["lon"], places["lat"])
    on_map = (xs >= 0) & (xs < grid.width_px) & (ys >= 0) & (ys < grid.height_px)
    label = np.zeros(len(xs), dtype=np.int64)
    label[on_map] = labels[ys[on_map].astype(int), xs[on_map].astype(int)]

    placed, *_ = settlements.prepare(map_dir=map_dir)
    tree = cKDTree(np.array([s.px for s in placed]))
    points = np.column_stack([xs, ys])
    distance, _ = tree.query(points)
    keep = (label > 0) & (distance * km_per_px > MIN_SETTLEMENT_DISTANCE_KM)
    # Stable order by GeoNames id so the shuffle does not depend on file order.
    kept = np.nonzero(keep)[0]
    kept = kept[np.argsort(places["id"][kept], kind="stable")]
    points = points[kept]
    province_of = np.array([province_by_index[int(i)] for i in label[kept]])

    provinces = settlements.load_provinces()
    area_px = {pid: float(props["area_px"]) for pid, props in geometry.items()}
    population = {pid: province_population(p) for pid, p in provinces.items()}
    quotas = province_quotas(area_px, population, TARGET_COUNT)
    area_km2 = {pid: area * km_per_px**2 for pid, area in area_px.items()}
    chosen = select(points, province_of, quotas, area_km2, km_per_px)

    hamlets = [
        {
            "name": str(places["name"][kept[i]]),
            "px": [round(float(points[i, 0]), 1), round(float(points[i, 1]), 1)],
            "province": str(province_of[i]),
        }
        for i in chosen
    ]
    hamlets.sort(key=lambda h: (h["province"], h["name"], h["px"]))
    path = map_dir / HAMLETS_FILE
    path.write_text(
        "[\n"
        + ",\n".join(json.dumps(h, ensure_ascii=False) for h in hamlets)
        + "\n]\n",
        encoding="utf-8",
    )
    return HamletResult(
        path=path,
        count=len(hamlets),
        candidates=len(kept),
        provinces=len({h["province"] for h in hamlets}),
        seconds=time.perf_counter() - started,
    )
