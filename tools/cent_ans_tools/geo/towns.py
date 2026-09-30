"""Footprints of the ordinary towns around 1340 (lot ZG6, ADR 0036).

``cent-ans geo towns`` (after ``geo anchors-fine``) reads ``data/rules/town_footprint.json``
(sourced densities, reference populations, walls, plan settings), the settlements of
``data/settlements/``, their refined anchors (``data/map/fine_anchors.json``), the roads
(``roads.geojson``), the fine rivers and the relief pyramid, and writes
``data/map/towns_1340.json``: for every settlement that is not an emblematic city
(``data/landmarks/``), an estimated population, an intra-muros area, an enclosure polygon fitted
to the fine relief (spur edges, river bank, coast), gates on the access roads, faubourgs, the
river and bridge crossing the site, monuments (castle, cathedral, abbey, windmill) and the radius
of the fields (``finage``) for the close-range field pattern of lot ZG5b.

Local coordinates are metres relative to the refined anchor, ``x`` towards +X of the map
(east) and ``y`` towards +Y (south), i.e. the world axes X and Z. Heights are metres on the
finest cached relief level (not exaggerated). Rendering data only: no game rule reads it.
"""

from __future__ import annotations

import json
import math
import time
import zlib
from collections.abc import Callable
from dataclasses import dataclass
from datetime import UTC, datetime
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[3]
DATA_DIR = ROOT / "data"
MAP_DIR = DATA_DIR / "map"
RULES_FILE = DATA_DIR / "rules" / "town_footprint.json"
OUT_FILE = "towns_1340.json"
KINDS = ("city", "town", "castle", "abbey", "village")
ROAD_PRIORITY = {"main": 0, "secondary": 1, "computed": 2}

HeightFn = Callable[[np.ndarray, np.ndarray], np.ndarray]
"""Heights (m) at local points ``(x east, y south)`` in metres; NaN off the map."""
WaterFn = Callable[[np.ndarray, np.ndarray], np.ndarray]
"""True where a local point is sea (or off the land mask)."""


# ------------------------------------------------------------------ population


def load_rules(path: Path = RULES_FILE) -> dict:
    """The town footprint rules (``data/rules/town_footprint.json``)."""
    return json.loads(path.read_text(encoding="utf-8"))


def seed_of(settlement_id: str) -> int:
    """Deterministic 31-bit seed of a settlement (same everywhere: CRC-32 of the id)."""
    return zlib.crc32(settlement_id.encode("utf-8")) & 0x7FFFFFFF


def population_estimate(settlement: dict, rules: dict) -> tuple[int, str]:
    """Population around 1340 and its basis (``reference:<source>`` or ``model``)."""
    model = rules["population_model"]
    for ref in rules["references"]:
        if ref["settlement"] == settlement["id"]:
            if "population" in ref:
                return int(ref["population"]), f"reference:{ref['source']}"
            factor = model["poll_tax_1377_factor"]
            return int(round(ref["poll_tax_1377"] * factor, -1)), (
                f"reference:{ref['source']}"
            )
    kind = model["kinds"][settlement["kind"]]
    value = kind["base"] * math.exp(
        kind["slope"] * (settlement["weight"] - kind["at_weight"])
    )
    for building in settlement.get("buildings", []):
        value *= model["building_factors"].get(building, 1.0)
    value = min(max(value, kind["min"]), kind["max"])
    return int(round(value, -1)), "model"


def density_for(population: float, rules: dict) -> float:
    """Built-up density (inhabitants per hectare), interpolated in log of the population."""
    table = rules["density_by_population"]
    pops = np.log([row[0] for row in table])
    dens = [row[1] for row in table]
    return float(np.interp(math.log(max(population, 1.0)), pops, dens))


def wall_kind(settlement: dict, rules: dict) -> str:
    """``stone``, ``palisade`` or ``none`` (castles and abbeys carry their own enclosure)."""
    buildings = settlement.get("buildings", [])
    if "bld_stone_walls" in buildings:
        return "stone"
    if "bld_palisade" in buildings:
        return "palisade"
    level = rules["walls"]["stone_from_fortification"].get(settlement["kind"])
    if level is not None and settlement.get("fortification_level", 0) >= level:
        return "stone"
    return "none"


@dataclass(frozen=True)
class Areas:
    """Surfaces of a settlement (hectares) and its split inside / outside the core."""

    population: int
    density: float
    built_ha: float
    core_ha: float
    core_population: int
    outside_population: int


def areas(
    settlement: dict, population: int, walls: str, rules: dict, ref: dict | None
) -> Areas:
    """Built-up area, core (walled or open) area and the population in the faubourgs."""
    kind = settlement["kind"]
    density = density_for(population, rules)
    built = population / density
    wall_rules = rules["walls"]
    if walls != "none":
        share = wall_rules["intra_muros_share"][kind] or wall_rules["open_core_share"]
        core = built * share * wall_rules["garden_margin"][kind]
    else:
        share = 1.0 if kind == "village" else wall_rules["open_core_share"]
        core = built * share
    if ref is not None and walls != "none":
        if "walled_area_ha" in ref:
            core = float(ref["walled_area_ha"])
        elif "wall_perimeter_km" in ref:
            # Compactness of a real enclosure ~0.75 of the disc of the same perimeter.
            perimeter = ref["wall_perimeter_km"] * 1000.0
            core = 0.75 * perimeter**2 / (4.0 * math.pi) / 10_000.0
    core_population = int(round(population * share))
    return Areas(
        population=population,
        density=round(density, 1),
        built_ha=round(built, 2),
        core_ha=round(core, 2),
        core_population=core_population,
        outside_population=population - core_population,
    )


# ------------------------------------------------------------------ geometry


def bearings(count: int) -> np.ndarray:
    """``count`` bearings (radians), 0 = +x (east), increasing towards +y (south)."""
    return np.arange(count) * (2.0 * math.pi / count)


def ground_grid(height: HeightFn, radii: np.ndarray, z_centre: float) -> list[float]:
    """Ground heights (m): centre, then rings at 0.5 and 1.0 x ``radii`` (same bearings).

    Flat list of ``1 + 2 * len(radii)`` values rounded to 0.1 m; a point the relief
    pyramid does not cover falls back on ``z_centre``.
    """
    angles = bearings(len(radii))
    values = [z_centre]
    for factor in (0.5, 1.0):
        heights = np.asarray(
            height(np.cos(angles) * radii * factor, np.sin(angles) * radii * factor),
            dtype=np.float64,
        )
        values.extend(np.where(np.isfinite(heights), heights, z_centre).tolist())
    return [round(float(v), 1) for v in values]


def polygon_area(radii: np.ndarray) -> float:
    """Area (m²) of the star polygon of ``radii`` on evenly spaced bearings."""
    n = len(radii)
    return float(0.5 * math.sin(2.0 * math.pi / n) * np.sum(radii * np.roll(radii, -1)))


def smooth_circular(values: np.ndarray, passes: int = 2) -> np.ndarray:
    """[1 2 1] smoothing on a closed ring."""
    out = values.astype(np.float64)
    for _ in range(passes):
        out = 0.25 * np.roll(out, 1) + 0.5 * out + 0.25 * np.roll(out, -1)
    return out


def relief_edges(
    height: HeightFn, r0: float, site: dict, step: float = 20.0
) -> tuple[np.ndarray, np.ndarray]:
    """Per bearing: radius of the first break of slope (NaN if none), and its drop."""
    angles = bearings(site["bearings"])
    radii = np.arange(step, r0 * site["max_radius_factor"] + step, step)
    xs = np.cos(angles)[:, None] * radii[None, :]
    ys = np.sin(angles)[:, None] * radii[None, :]
    h = height(xs, ys)
    h0 = float(np.nanmedian(height(np.zeros(1), np.zeros(1))))
    edges = np.full(len(angles), np.nan)
    drops = np.zeros(len(angles))
    if not math.isfinite(h0):
        return edges, drops
    start = r0 * site["min_radius_factor"] * 0.8
    for b in range(len(angles)):
        profile = h[b]
        for i in range(1, len(radii)):
            if radii[i] < start or not np.isfinite(profile[i]):
                continue
            grade = (profile[i - 1] - profile[i]) / step
            drop = h0 - profile[i]
            if grade > site["edge_slope"] and drop > site["edge_drop_m"]:
                edges[b] = radii[i - 1]
                drops[b] = drop
                break
    return edges, drops


def river_bank_limits(
    river: dict | None, angles: np.ndarray, r_max: float, margin: float = 15.0
) -> np.ndarray:
    """Per bearing: distance to the near bank of a straight river reach (inf if none)."""
    out = np.full(len(angles), np.inf)
    if river is None:
        return out
    p = np.asarray(river["point"], dtype=np.float64)
    t = np.asarray(river["tangent"], dtype=np.float64)
    t = t / max(np.hypot(*t), 1e-9)
    half = 0.5 * river["width_m"] + margin
    s0 = float(
        t[0] * (0.0 - p[1]) - t[1] * (0.0 - p[0])
    )  # signed distance of the anchor
    for b, angle in enumerate(angles):
        u = np.array([math.cos(angle), math.sin(angle)])
        rate = float(t[0] * u[1] - t[1] * u[0])
        if abs(rate) < 1e-6 or rate * s0 >= 0.0:
            continue  # parallel, or moving away from the river
        reach = (abs(s0) - half) / abs(rate)
        if 0.0 < reach < r_max:
            out[b] = reach
    return out


def enclosure_radii(
    height: HeightFn,
    water: WaterFn | None,
    core_m2: float,
    river: dict | None,
    clip_river: bool,
    site: dict,
    rng: np.random.Generator,
) -> tuple[np.ndarray, str]:
    """Radii of the enclosure (or built-up core) and the site class.

    The polygon follows the break of slope where one exists (spur, hill town), is cut by
    the near river bank for small towns and by the sea, keeps some irregularity and is
    scaled back to the target area on its free bearings.
    """
    angles = bearings(site["bearings"])
    r0 = math.sqrt(core_m2 / math.pi)
    lo, hi = site["min_radius_factor"] * r0, site["max_radius_factor"] * r0
    edges, _ = relief_edges(height, r0, site)
    has_edge = np.isfinite(edges)
    limit = np.full(len(angles), np.inf)
    limit[has_edge] = edges[has_edge]
    bank = river_bank_limits(river, angles, hi) if clip_river else limit * np.inf
    limit = np.minimum(limit, bank)
    if water is not None:
        steps = np.linspace(0.3, site["max_radius_factor"], 12) * r0
        xs = np.cos(angles)[:, None] * steps[None, :]
        ys = np.sin(angles)[:, None] * steps[None, :]
        wet = water(xs, ys)
        for b in range(len(angles)):
            hit = np.nonzero(wet[b])[0]
            if len(hit):
                limit[b] = min(limit[b], steps[hit[0]] - 0.5 * (steps[1] - steps[0]))
    noise = smooth_circular(rng.normal(0.0, 1.0, len(angles)), 3)
    noise = noise / max(float(np.std(noise)), 1e-6) * site["irregularity"]
    radii = r0 * (1.0 + noise)
    radii = np.minimum(radii, limit)
    radii = np.clip(radii, lo * 0.5, hi)
    # Scale the unconstrained bearings back to the target area (a few passes).
    for _ in range(8):
        area = polygon_area(radii)
        if area <= 0.0:
            break
        free = radii < limit - 1.0
        if not free.any():
            break
        factor = math.sqrt(core_m2 / area)
        if abs(factor - 1.0) < 0.01:
            break
        radii[free] = np.minimum(np.clip(radii[free] * factor, lo, hi), limit[free])
    radii = np.maximum(smooth_circular(radii, 1), lo * 0.5)
    constrained = has_edge.mean()
    if constrained >= site["spur_share"]:
        site_class = "spur"
    elif river is not None and np.isfinite(bank).any():
        site_class = "river"
    elif water is not None and np.isfinite(limit).any() and not has_edge.any():
        site_class = "coast"
    else:
        site_class = "plain"
    return radii, site_class


def road_bearings(
    lines: list[tuple[str, np.ndarray]], radius: float, max_count: int
) -> list[dict]:
    """Bearings where the local road ``lines`` leave the circle of ``radius`` (m).

    ``lines`` are ``(type, points (n, 2))`` in local metres. Close exits (< 25°) are merged,
    keeping the most important road; at most ``max_count`` exits.
    """
    exits: list[tuple[float, str]] = []
    for road_type, pts in lines:
        d = np.hypot(pts[:, 0], pts[:, 1])
        for i in range(len(pts) - 1):
            if (d[i] - radius) * (d[i + 1] - radius) > 0.0:
                continue
            a, b = pts[i], pts[i + 1]
            da, db = d[i], d[i + 1]
            t = 0.5 if abs(db - da) < 1e-9 else (radius - da) / (db - da)
            p = a + (b - a) * min(max(t, 0.0), 1.0)
            exits.append((math.atan2(p[1], p[0]) % (2 * math.pi), road_type))
    exits.sort(key=lambda e: (ROAD_PRIORITY.get(e[1], 3), e[0]))
    kept: list[dict] = []
    for angle, road_type in exits:
        if any(
            abs((angle - k["bearing"] + math.pi) % (2 * math.pi) - math.pi)
            < math.radians(25)
            for k in kept
        ):
            continue
        kept.append({"bearing": angle, "type": road_type})
        if len(kept) >= max_count:
            break
    kept.sort(key=lambda k: k["bearing"])
    return kept


def complete_gates(gates: list[dict], min_count: int, seed: int) -> list[dict]:
    """Add synthetic tracks until ``min_count`` exits (opposite the largest road first)."""
    out = list(gates)
    base = (seed % 360) * math.pi / 180.0 if not out else out[0]["bearing"]
    while len(out) < min_count:
        if not out:
            out.append({"bearing": base, "type": "track"})
            continue
        angles = sorted(g["bearing"] for g in out)
        gaps = [
            ((angles[(i + 1) % len(angles)] - angles[i]) % (2 * math.pi) or 2 * math.pi)
            for i in range(len(angles))
        ]
        i = int(np.argmax(gaps))
        out.append(
            {"bearing": (angles[i] + gaps[i] / 2.0) % (2 * math.pi), "type": "track"}
        )
    out.sort(key=lambda g: g["bearing"])
    return out


def radius_at(radii: np.ndarray, angle: float) -> float:
    """Polygon radius at ``angle`` (linear between bearings)."""
    n = len(radii)
    f = (angle % (2 * math.pi)) / (2 * math.pi) * n
    i = int(f) % n
    t = f - int(f)
    return float(radii[i] * (1 - t) + radii[(i + 1) % n] * t)


def faubourgs(
    gates: list[dict], radii: np.ndarray, outside_population: int, rules: dict
) -> list[dict]:
    """Faubourg along each gate road: bearing, start radius and length (m)."""
    if outside_population <= 0 or not gates:
        return []
    fr = rules["faubourgs"]
    weights = np.array(
        [fr["main_road_weight"] if g["type"] == "main" else 1.0 for g in gates]
    )
    weights = weights / weights.sum()
    out = []
    for gate, w in zip(gates, weights, strict=True):
        share = outside_population * w
        length = fr["length_per_sqrt_pop_m"] * math.sqrt(share)
        if length < fr["min_length_m"] * 0.5:
            continue
        length = min(max(length, fr["min_length_m"]), fr["max_length_m"])
        out.append(
            {
                "bearing": round(math.degrees(gate["bearing"]), 1),
                "start_m": round(radius_at(radii, gate["bearing"]), 1),
                "length_m": round(length, 1),
                "population": int(round(share)),
            }
        )
    return out


def best_point(
    height: HeightFn,
    candidates: np.ndarray,
    prefer_high: bool = True,
    distance_weight: float = 0.0,
) -> np.ndarray | None:
    """Highest (or lowest) candidate, with a penalty per metre from the anchor."""
    if len(candidates) == 0:
        return None
    h = height(candidates[:, 0], candidates[:, 1])
    score = np.where(np.isfinite(h), h if prefer_high else -h, -np.inf)
    score = score - distance_weight * np.hypot(candidates[:, 0], candidates[:, 1])
    if not np.isfinite(score).any():
        return None
    return candidates[int(np.argmax(score))]


def ring_candidates(
    radii: np.ndarray, inner: float, outer: float, count: int = 5
) -> np.ndarray:
    """Points between ``inner`` and ``outer`` × the polygon radius on every bearing."""
    angles = bearings(len(radii))
    pts = []
    for f in np.linspace(inner, outer, count):
        pts.append(
            np.column_stack([np.cos(angles) * radii * f, np.sin(angles) * radii * f])
        )
    return np.concatenate(pts)


def monuments(
    settlement: dict,
    height: HeightFn,
    radii: np.ndarray,
    gates: list[dict],
    population: int,
    rng: np.random.Generator,
) -> list[dict]:
    """Castle, cathedral, abbey, market hall and windmill of the settlement (local m)."""
    kind = settlement["kind"]
    buildings = settlement.get("buildings", [])
    out: list[dict] = []

    def add(name: str, point: np.ndarray | None, size: float, **extra) -> None:
        if point is None:
            return
        entry = {
            "kind": name,
            "at": [round(float(point[0]), 1), round(float(point[1]), 1)],
            "size_m": round(size, 1),
            "yaw": round(float(rng.uniform(-math.pi, math.pi)), 3),
        }
        entry.update(extra)
        out.append(entry)

    if kind == "castle":
        cand = ring_candidates(radii, 0.0, 0.8, 5)
        add(
            "castle",
            best_point(height, cand, distance_weight=0.02),
            55.0 + 5.0 * settlement.get("fortification_level", 2),
            keep=True,
        )
    elif "bld_castle" in buildings:
        add(
            "castle",
            best_point(height, ring_candidates(radii, 0.6, 0.9, 4)),
            70.0,
            keep=True,
        )
    if kind == "abbey":
        add("abbey", np.zeros(2), 140.0)
    elif "bld_abbey" in buildings and kind in ("city", "town"):
        # A suburban abbey in the largest gap between the gates, just outside the core.
        angles = sorted(g["bearing"] for g in gates) or [0.0]
        gaps = [
            ((angles[(i + 1) % len(angles)] - angles[i]) % (2 * math.pi) or 2 * math.pi)
            for i in range(len(angles))
        ]
        i = int(np.argmax(gaps))
        angle = angles[i] + gaps[i] / 2.0
        r = radius_at(radii, angle) * 1.15 + 60.0
        add("abbey", np.array([math.cos(angle) * r, math.sin(angle) * r]), 110.0)
    if "bld_cathedral" in buildings:
        cand = ring_candidates(radii, 0.15, 0.45, 4)
        length = min(max(45.0 + population / 1000.0 * 2.5, 60.0), 130.0)
        add("cathedral", best_point(height, cand, distance_weight=0.03), length)
    trade = ("bld_guild_hall", "bld_weaving_workshop", "bld_fair")
    if kind == "city" or (kind == "town" and any(b in buildings for b in trade)):
        add("hall", np.zeros(2), 23.0)
    if "bld_windmill" in buildings:
        cand = ring_candidates(radii, 1.3, 2.2, 4)
        add("windmill", best_point(height, cand, distance_weight=0.01), 7.0)
    return out


#: A river at least this many times wider than the nearest watercourse takes its place
#: (RS-G: Tours kept a 15 m brook between the town and the Loire, 400 m wide, whose banks
#: then got faubourg houses).
MAJOR_RIVER_RATIO = 4.0


def pick_river_feature(candidates: dict[int, tuple[float, float]]) -> int | None:
    """River feature of a town: the nearest, unless a much wider one is also in reach.

    Args:
        candidates: feature index -> (distance to the town centre in m, median width in m)
            for every fine river feature within the search radius.

    Returns:
        The chosen feature index, or ``None`` without candidates.
    """
    if not candidates:
        return None
    nearest = min(candidates, key=lambda f: (candidates[f][0], f))
    widest = max(candidates, key=lambda f: (candidates[f][1], -candidates[f][0], -f))
    if candidates[widest][1] >= MAJOR_RIVER_RATIO * candidates[nearest][1]:
        return widest
    return nearest


def river_reach(
    points: np.ndarray, widths: np.ndarray, radius: float, spacing: float = 30.0
) -> dict | None:
    """Local reach of the nearest river: ordered polyline (m), width and tangent."""
    if len(points) < 2:
        return None
    d = np.hypot(points[:, 0], points[:, 1])
    k = int(np.argmin(d))
    if d[k] > radius:
        return None
    near = d <= radius
    pts = points[near]
    wid = widths[near]
    # Principal direction of the reach, ordering by projection.
    centred = pts - pts.mean(axis=0)
    _, _, vt = np.linalg.svd(centred, full_matrices=False)
    t = vt[0]
    order = np.argsort(pts @ t)
    pts, wid = pts[order], wid[order]
    along = np.concatenate([[0.0], np.cumsum(np.hypot(*np.diff(pts, axis=0).T))])
    if along[-1] < spacing:
        return None
    samples = np.arange(0.0, along[-1] + 1e-6, spacing)
    line = np.column_stack(
        [np.interp(samples, along, pts[:, 0]), np.interp(samples, along, pts[:, 1])]
    )
    closest = pts[int(np.argmin(np.hypot(pts[:, 0], pts[:, 1])))]
    return {
        "point": [round(float(closest[0]), 1), round(float(closest[1]), 1)],
        "tangent": [round(float(t[0]), 4), round(float(t[1]), 4)],
        "width_m": round(float(np.median(wid)), 1),
        "line": [[round(float(x), 1), round(float(y), 1)] for x, y in line],
    }


def town_bridge_allowed(crossing: dict, rules: dict) -> bool:
    """Whether a river crossing gets a fixed bridge in the 1:1 town.

    Historical bridges always do; fords and ferries never (Cologne had no bridge over the
    Rhine before the 19th century); generic road crossings only over narrow rivers.
    """
    kind = str(crossing["id"]).split("_", 1)[0]
    if kind == "bridge":
        return True
    if kind == "road":
        return float(crossing["width_m"]) <= rules["site"]["road_bridge_max_width_m"]
    return False


def parish_count(settlement: dict, population: int, rules: dict) -> int:
    """Parish churches: one per ``parish_inhabitants`` in towns, one in a village or bourg."""
    if settlement["kind"] in ("city", "town"):
        count = int(round(population / rules["plan"]["parish_inhabitants"]))
        return max(1, min(rules["plan"]["max_parishes"], count))
    return 0 if settlement["kind"] == "abbey" else 1


def plan_town(
    settlement: dict,
    rules: dict,
    height: HeightFn,
    water: WaterFn | None = None,
    road_lines: list[tuple[str, np.ndarray]] | None = None,
    river: dict | None = None,
    bridge: dict | None = None,
) -> dict:
    """Footprint of one settlement (everything local, in metres)."""
    seed = seed_of(settlement["id"])
    rng = np.random.default_rng(seed)
    population, basis = population_estimate(settlement, rules)
    ref = next(
        (r for r in rules["references"] if r["settlement"] == settlement["id"]), None
    )
    walls = wall_kind(settlement, rules)
    area = areas(settlement, population, walls, rules, ref)
    site = rules["site"]
    clip_river = population < site["both_banks_population"]
    radii, site_class = enclosure_radii(
        height, water, area.core_ha * 10_000.0, river, clip_river, site, rng
    )
    wall_rules = rules["walls"]
    exit_radius = float(np.max(radii)) * 1.15 + 60.0
    gates = road_bearings(road_lines or [], exit_radius, wall_rules["max_gates"])
    min_gates = wall_rules["min_gates"] if settlement["kind"] != "village" else 1
    gates = complete_gates(gates, min_gates, seed)
    fin = rules["finage"]
    finage = min(
        max(fin["radius_per_sqrt_pop_m"] * math.sqrt(population), fin["min_radius_m"]),
        fin["max_radius_m"],
    )
    angles = bearings(len(radii))
    h_poly = height(np.cos(angles) * radii, np.sin(angles) * radii)
    h0 = height(np.zeros(1), np.zeros(1))
    z_centre = float(h0[0]) if np.isfinite(h0[0]) else 0.0
    return {
        "id": settlement["id"],
        "kind": settlement["kind"],
        "seed": seed,
        "population": population,
        "population_basis": basis,
        "density": area.density,
        "households": int(round(population / rules["persons_per_household"])),
        "built_ha": area.built_ha,
        "core_ha": round(polygon_area(radii) / 10_000.0, 2),
        "core_population": area.core_population,
        "walls": walls,
        "site": site_class,
        "relief_m": round(float(np.nanmax(h_poly) - np.nanmin(h_poly)), 1)
        if np.isfinite(h_poly).any()
        else 0.0,
        "z_m": round(float(h0[0]), 2) if np.isfinite(h0[0]) else 0.0,
        "radii": [round(float(r), 1) for r in radii],
        "ground_m": ground_grid(height, radii, z_centre),
        "gates": [
            {"bearing": round(math.degrees(g["bearing"]), 1), "type": g["type"]}
            for g in gates
        ],
        "faubourgs": faubourgs(gates, radii, area.outside_population, rules),
        "monuments": monuments(settlement, height, radii, gates, population, rng),
        "parishes": parish_count(settlement, population, rules),
        "river": river,
        "bridge": bridge,
        "finage_radius_m": round(finage, 0),
    }


# ------------------------------------------------------------------ inputs


def load_settlements(data_dir: Path = DATA_DIR) -> list[dict]:
    """All settlements of ``data/settlements/prov_*.json``."""
    out = []
    for path in sorted((data_dir / "settlements").glob("prov_*.json")):
        out.extend(json.loads(path.read_text(encoding="utf-8")))
    return out


def landmark_settlements(data_dir: Path = DATA_DIR) -> set[str]:
    """Settlement ids of the emblematic cities (lot VH, left to their own models)."""
    ids = set()
    for path in sorted((data_dir / "landmarks").glob("*.json")):
        ids.add(json.loads(path.read_text(encoding="utf-8"))["settlement"])
    return ids


def local_roads(
    roads: list[tuple[str, np.ndarray]],
    centre_units: np.ndarray,
    mpp: float,
    radius_m: float,
) -> list[tuple[str, np.ndarray]]:
    """Roads (world units) near a centre, converted to local metres."""
    out = []
    r_units = radius_m / mpp
    for road_type, pts in roads:
        lo, hi = pts.min(axis=0), pts.max(axis=0)
        if (
            lo[0] > centre_units[0] + r_units
            or hi[0] < centre_units[0] - r_units
            or lo[1] > centre_units[1] + r_units
            or hi[1] < centre_units[1] - r_units
        ):
            continue
        out.append((road_type, (pts - centre_units) * mpp))
    return out


def load_roads(map_dir: Path) -> list[tuple[str, np.ndarray]]:
    """``roads.geojson`` lines in world units."""
    data = json.loads((map_dir / "roads.geojson").read_text(encoding="utf-8"))
    out = []
    for feature in data["features"]:
        geom = feature["geometry"]
        road_type = feature["properties"].get("type", "secondary")
        lines = (
            [geom["coordinates"]]
            if geom["type"] == "LineString"
            else geom["coordinates"]
            if geom["type"] == "MultiLineString"
            else []
        )
        for coords in lines:
            if len(coords) >= 2:
                out.append((road_type, np.asarray(coords, dtype=np.float64)))
    return out


@dataclass
class TownsResult:
    """Summary of ``build``."""

    path: Path
    towns: int
    walled: int
    spur: int
    river: int
    bridges: int
    population: int
    seconds: float

    def summary(self) -> str:
        """One line for the console."""
        return (
            f"{self.path} : {self.towns} colonies ({self.walled} enceintes, "
            f"{self.spur} sur éperon, {self.river} au bord d'un fleuve, {self.bridges} ponts), "
            f"{self.population:,} habitants estimés ({self.seconds:.0f} s)"
        )


def build(
    map_dir: Path = MAP_DIR,
    data_dir: Path = DATA_DIR,
    rules_path: Path = RULES_FILE,
    log=print,  # noqa: ANN001
) -> TownsResult:
    """Write ``data/map/towns_1340.json`` from the rules, anchors, roads and relief."""
    from PIL import Image

    from cent_ans_tools.geo import fine_anchors, fine_relief, hydro_fine, pyramid

    pyramid.require_world_frame(map_dir, "geo towns")
    started = time.time()
    rules = load_rules(rules_path)
    frame = fine_anchors.Frame(pyramid.map_bounds(map_dir))
    mpp = frame.mpp
    relief = fine_relief.FineRelief(map_dir, frame.bounds)
    anchors = json.loads((map_dir / "fine_anchors.json").read_text(encoding="utf-8"))
    features_path = (
        map_dir
        / pyramid.PYRAMID_DIR_NAME
        / hydro_fine.TILES_SUBDIR
        / hydro_fine.FEATURES_FILE
    )
    rivers = None
    if features_path.exists():
        names = [
            f["name"]
            for f in json.loads(features_path.read_text(encoding="utf-8"))["features"]
        ]
        rivers = fine_anchors.RiverIndex(map_dir, frame, names, cache=64)
    land = np.asarray(Image.open(map_dir / "land_mask.png"), dtype=np.float32) / 255.0
    roads = load_roads(map_dir)
    crossings = [
        c
        for c in anchors["crossings"]
        if c.get("snapped") and town_bridge_allowed(c, rules)
    ]
    crossing_px = (
        np.array([c["px"] for c in crossings]) if crossings else np.zeros((0, 2))
    )
    excluded = landmark_settlements(data_dir)
    towns = {}
    for settlement in sorted(load_settlements(data_dir), key=lambda s: s["id"]):
        sid = settlement["id"]
        if sid in excluded or sid not in anchors["settlements"]:
            continue
        centre_units = np.asarray(anchors["settlements"][sid]["px"], dtype=np.float64)
        centre_m = frame.to_m(centre_units)[0]

        def height(x, y, c=centre_m):  # noqa: ANN001, ANN202
            x = np.asarray(x, dtype=np.float64)
            y = np.asarray(y, dtype=np.float64)
            return relief.sample(c[0] + x, c[1] - y)

        def water(x, y, c=centre_units):  # noqa: ANN001, ANN202
            ux = np.clip(c[0] + np.asarray(x) / mpp, 0, land.shape[1] - 1)
            uy = np.clip(c[1] + np.asarray(y) / mpp, 0, land.shape[0] - 1)
            return land[uy.astype(int), ux.astype(int)] < 0.5

        population, _ = population_estimate(settlement, rules)
        guess_r = math.sqrt(
            population / density_for(population, rules) * 10_000 / math.pi
        )
        search = max(guess_r * rules["site"]["river_search_factor"], 400.0)
        river = None
        if rivers is not None:
            candidates: dict[int, list] = {}
            for tile in rivers.tiles_around(centre_m.reshape(1, 2), search):
                for i in tile["tree"].query_ball_point(centre_m, search):
                    candidates.setdefault(int(tile["feature"][i]), []).append(
                        (tile["points"][i], float(tile["width"][i]))
                    )
            feature = pick_river_feature(
                {
                    f: (
                        float(min(np.hypot(*(p - centre_m)) for p, _ in items)),
                        float(np.median([w for _, w in items])),
                    )
                    for f, items in candidates.items()
                }
            )
            if feature is not None:
                items = candidates[feature]
                local = np.array([p for p, _ in items]) - centre_m
                river = river_reach(
                    np.column_stack([local[:, 0], -local[:, 1]]),
                    np.array([w for _, w in items]),
                    search,
                )
        bridge = None
        if len(crossing_px):
            d = np.hypot(*(crossing_px - centre_units).T) * mpp
            k = int(np.argmin(d))
            if d[k] < max(guess_r * 1.8, 350.0):
                c = crossings[k]
                offset = (np.asarray(c["px"]) - centre_units) * mpp
                bridge = {
                    "id": c["id"],
                    "at": [round(float(offset[0]), 1), round(float(offset[1]), 1)],
                    "dir": c["dir"],
                    "width_m": c["width_m"],
                    "z_deck": c["z_deck"],
                    "z_water": c["z_water"],
                }
        lines = local_roads(roads, centre_units, mpp, guess_r * 3.0 + 600.0)
        town = plan_town(settlement, rules, height, water, lines, river, bridge)
        town["px"] = [round(float(v), 4) for v in centre_units]
        towns[sid] = town
    payload = {
        "description": "Emprise des villes ordinaires vers 1340 (lot ZG6, ADR 0036). Généré par `cent-ans geo towns` depuis data/rules/town_footprint.json : ne pas modifier à la main. Coordonnées locales en mètres depuis l'ancrage fin (x vers l'est = +X monde, y vers le sud = +Z monde), hauteurs en mètres (relief fin, non exagéré). Rendu seulement, aucune règle de jeu.",
        "version": 1,
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat(),
        "year": rules["year"],
        "meters_per_unit": round(mpp, 4),
        "bearings": rules["site"]["bearings"],
        "walls": {k: rules["walls"][k] for k in ("stone", "palisade", "gate_depth_m")},
        "plan": {k: v for k, v in rules["plan"].items() if k != "description"},
        "towns": towns,
    }
    path = map_dir / OUT_FILE
    path.write_text(
        json.dumps(payload, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    values = list(towns.values())
    log(f"villes : {len(values)}")
    return TownsResult(
        path=path,
        towns=len(values),
        walled=sum(t["walls"] != "none" for t in values),
        spur=sum(t["site"] == "spur" for t in values),
        river=sum(t["river"] is not None for t in values),
        bridges=sum(t["bridge"] is not None for t in values),
        population=sum(t["population"] for t in values),
        seconds=time.time() - started,
    )
