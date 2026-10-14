"""Généralisation de la carte composée (lot CC, C0b).

Tout part d'un :class:`Frame` : un raster de la carte composée à l'échelle
``scale`` (pixels de cadre par pixel pleine résolution) dont on connaît, pour
chaque pixel, l'origine réelle (déformation inverse). Les rasters réels (terre,
masses montagneuses, provinces) y sont rééchantillonnés ; les vecteurs (fleuves,
îles à garder) passent par la déformation avant. Sorties en pixels composés
pleine résolution (1 px = 719 m nominaux).
"""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from shapely.geometry import LineString, MultiPolygon, Point, Polygon, shape
from shapely.ops import unary_union
from skimage.morphology import skeletonize

from cent_ans_tools.compose.warp import (
    COMPOSE_DIR,
    CROP_LONLAT,
    KM_PER_PX,
    MAP_DIR,
    Warp,
    real_path,
)
from cent_ans_tools.geo.navgrid import massif_mask
from cent_ans_tools.paths import DATA_DIR

Image.MAX_IMAGE_PIXELS = None

CACHE_DIR = Path.home() / "dev" / "cent-ans-raw" / "compose" / "cache"

#: Îles à garder (point lon/lat) même minuscules, gonflées à une aire lisible.
KEEP_ISLANDS: dict[str, tuple[float, float]] = {
    "Corse": (9.1, 42.1),
    "Sardaigne": (9.0, 40.0),
    "Sicile": (14.0, 37.6),
    "Crète": (24.9, 35.2),
    "Chypre": (33.0, 35.0),
    "Rhodes": (28.0, 36.2),
    "Majorque": (2.95, 39.6),
    "Minorque": (4.0, 40.0),
    "Malte": (14.4, 35.9),
    "Wight": (-1.3, 50.67),
    "Man": (-4.5, 54.2),
    "Anglesey": (-4.37, 53.28),
    "Gotland": (18.5, 57.5),
    "Seeland": (11.8, 55.5),
    "Fionie": (10.35, 55.35),
    "Eubée (Négrepont)": (23.9, 38.55),
    "Corfou": (19.7, 39.6),
    "Leucade": (20.7, 38.7),
    "Céphalonie": (20.55, 38.25),
    "Zante": (20.9, 37.8),
    "Orcades": (-3.0, 59.0),
    "Hébrides (Lewis)": (-6.6, 58.2),
    "Hébrides (Skye)": (-6.2, 57.3),
    "Lesbos": (26.3, 39.2),
    "Chios": (26.0, 38.4),
    "Rügen": (13.4, 54.45),
}

#: Fleuves toujours gardés s'ils existent (nom Natural Earth).
KEY_RIVERS = (
    "Loire",
    "Seine",
    "Rhône",
    "Rhine",
    "Garonne",
    "Thames",
    "Po",
    "Tiber",
    "Ebro",
    "Tajo",
    "Danube",
    "Elbe",
    "Oder",
    "Vistula",
    "Dnipro",
    "Don",
    "Maas",
    "Guadalquivir",
    "Duero",
    "Weser",
    "Dniester",
    "Sava",
    "Tisza",
)
RIVER_ALIASES = {"Donau": "Danube", "Tagus": "Tajo", "Dnieper": "Dnipro"}


# ----------------------------------------------------------------------------
# Cadre
# ----------------------------------------------------------------------------


@dataclass
class Frame:
    """Raster de la carte composée et son origine réelle (déformation inverse)."""

    warp: Warp
    scale: float
    height: int = 0
    width: int = 0
    src_px: np.ndarray = field(default=None, repr=False)  # (2, h, w) pixels réels
    inside: np.ndarray = field(default=None, repr=False)

    def __post_init__(self) -> None:
        """Calcule la taille de sortie et les coordonnées réelles de chaque pixel."""
        w, h = self.warp.size_px
        self.width, self.height = int(round(w * self.scale)), int(round(h * self.scale))
        step = 8
        gx = np.arange(0, self.width + step, step)
        gy = np.arange(0, self.height + step, step)
        cx, cy = np.meshgrid((gx + 0.5) / self.scale, (gy + 0.5) / self.scale)
        px, py = self.warp.inverse_px(cx, cy)
        lon, lat = self.warp.grid.pixel_to_lonlat(px, py)

        def up(a):
            z = ndimage.zoom(a, step, order=1)
            return z[: self.height, : self.width]

        # coordonnées de pixel de cadre -> indices du réseau grossier
        self.src_px = np.stack([up(px), up(py)]).astype(np.float32)
        lon, lat = up(lon), up(lat)
        lon0, lat0, lon1, lat1 = CROP_LONLAT
        self.inside = (lon >= lon0) & (lon <= lon1) & (lat >= lat0) & (lat <= lat1)

    def sample(self, source: np.ndarray, down: int, order: int = 1) -> np.ndarray:
        """Rééchantillonne un raster réel stocké à ``1/down`` de la pleine résolution."""
        x0, y0 = self.warp.box[:2]  # le raster source est la carte entière
        del x0, y0
        cols = self.src_px[0] / down - 0.5
        rows = self.src_px[1] / down - 0.5
        return ndimage.map_coordinates(
            source, [rows, cols], order=order, mode="nearest"
        )

    def to_composed(self, xy: np.ndarray) -> np.ndarray:
        """Pixels de cadre -> pixels composés pleine résolution."""
        return (np.asarray(xy, dtype=float) + 0.5) / self.scale


# ----------------------------------------------------------------------------
# Sources réelles (caches)
# ----------------------------------------------------------------------------


def _half(path: Path, mode: str | None = None) -> np.ndarray:
    img = Image.open(path)
    if mode:
        img = img.convert(mode)
    w, h = img.size
    return np.asarray(img.resize((w // 2, h // 2), Image.BOX), dtype=np.float32)


def land_half() -> np.ndarray:
    """Masque terre (0..1) à la demi-résolution."""
    return _half(real_path("land_mask.png")) / 255.0


def relief_masks_half() -> dict[str, np.ndarray]:
    """Masses montagneuses (règle ``massif_impassable``) et collines hautes, demi-résolution.

    Les rayons de la règle (en cellules pleine résolution) sont divisés par 2.
    """
    cache = CACHE_DIR / "relief_masks_half.npz"
    if cache.exists():
        z = np.load(cache)
        return {k: z[k] for k in z.files}
    meta = json.loads(real_path("map.json").read_text("utf-8"))
    lo, hi = meta["height_min_m"], meta["height_max_m"]
    img = Image.open(real_path("heightmap.png"))
    w, h = img.size
    raw = np.asarray(img.resize((w // 2, h // 2), Image.BOX), dtype=np.float32)
    height = lo + raw * (hi - lo) / 65535.0
    water = land_half() < 0.5
    rules = json.loads((DATA_DIR / "movement" / "rules.json").read_text("utf-8"))
    params = dict(rules["massif_impassable"])
    half_params = {
        **params,
        "base_radius_cells": max(1, params["base_radius_cells"] // 2),
        "smooth_cells": max(1, params["smooth_cells"] // 2),
    }
    massif = massif_mask(height, water, {"massif_impassable": half_params})
    hills_params = {**half_params, "min_m": 650.0, "min_relief_m": 300.0}
    hills = massif_mask(height, water, {"massif_impassable": hills_params}) & ~massif
    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    np.savez_compressed(cache, massif=massif, hills=hills)
    return {"massif": massif, "hills": hills}


def province_labels() -> np.ndarray:
    """``province_ids.png`` décodé (uint16, pleine résolution)."""
    rgb = np.asarray(Image.open(MAP_DIR / "province_ids.png"))
    return rgb[..., 0].astype(np.uint16) | (rgb[..., 1].astype(np.uint16) << 8)


def province_index_map() -> dict[int, str]:
    """Indice de l'image -> identifiant de province."""
    geo = json.loads((MAP_DIR / "provinces.geojson").read_text("utf-8"))
    return {f["properties"]["index"]: f["properties"]["id"] for f in geo["features"]}


def merge_table() -> dict[str, str]:
    """Province ancienne -> province fusionnée, d'après ``selection.json`` (si présent).

    Les provinces absentes de la table sont retirées (``selection.json`` existe).
    """
    path = COMPOSE_DIR / "selection.json"
    if not path.exists():
        return {}
    try:
        data = json.loads(path.read_text("utf-8"))
        items = data.get("provinces", []) if isinstance(data, dict) else []
        table = {}
        for item in items:
            for old in item.get("merged_from", []) or []:
                table[old] = item["id"]
        return table
    except (ValueError, KeyError, TypeError, AttributeError):
        return {}


# ----------------------------------------------------------------------------
# Terre et îles
# ----------------------------------------------------------------------------


def _polygons_from_mask(mask: np.ndarray, frame: Frame) -> list[Polygon]:
    from rasterio import features

    polys = []
    for geom, value in features.shapes(mask.astype(np.uint8), mask=mask):
        if value:
            polys.append(shape(geom))
    out = []
    for poly in polys:
        coords = lambda ring: frame.to_composed(np.array(ring.coords) - 0.5)  # noqa: E731
        out.append(Polygon(coords(poly.exterior), [coords(r) for r in poly.interiors]))
    return out


def generalise_land(
    frame: Frame,
    smooth_km: float = 14.0,
    simplify_km: float = 5.0,
    min_island_km2: float = 5000.0,
    min_kept_km2: float = 3500.0,
    min_lake_km2: float = 30000.0,
) -> tuple[MultiPolygon, dict]:
    """Terre généralisée : lissage, simplification, îles filtrées / gardées gonflées."""
    land = frame.sample(land_half(), 2, order=1)
    land = np.where(frame.inside, land, 0.0)
    sigma = smooth_km / KM_PER_PX * frame.scale
    smooth = ndimage.gaussian_filter(land, sigma)
    polys = _polygons_from_mask(smooth > 0.5, frame)
    px2_to_km2 = KM_PER_PX**2
    kept: list[Polygon] = []
    for poly in polys:
        holes = [
            h for h in poly.interiors if Polygon(h).area * px2_to_km2 >= min_lake_km2
        ]
        poly = Polygon(poly.exterior, holes)
        if poly.area * px2_to_km2 >= min_island_km2:
            kept.append(poly)
    stats = {"kept_big": len(kept), "islands": {}}
    for name, (lon, lat) in KEEP_ISLANDS.items():
        x, y = (float(v) for v in frame_forward(frame, lon, lat))
        target = Point(x, y)
        hit = next((i for i, p in enumerate(kept) if p.distance(target) < 25), None)
        if hit is None:
            near = [p for p in polys if p.distance(target) < 25]
            base = unary_union(near) if near else target.buffer(1.0)
            kept_poly = base
            status = "gonflée" if near else "créée"
        else:
            kept_poly = kept[hit]
            status = "conservée"
        area = kept_poly.area * px2_to_km2
        if area < min_kept_km2:
            r = 0.5
            while kept_poly.buffer(r).area * px2_to_km2 < min_kept_km2:
                r += 0.5
            kept_poly = kept_poly.buffer(r, resolution=8)
            if hit is not None:
                kept.pop(hit)
                kept.append(kept_poly)
            else:
                kept.append(kept_poly)
        elif hit is None:
            kept.append(kept_poly)
        stats["islands"][name] = status
    merged = unary_union([p.buffer(0) for p in kept]).simplify(simplify_km / KM_PER_PX)
    geoms = [merged] if isinstance(merged, Polygon) else list(merged.geoms)
    return MultiPolygon([g for g in geoms if isinstance(g, Polygon)]), stats


def frame_forward(frame: Frame, lon, lat):
    """``(lon, lat)`` -> pixels composés pleine résolution."""
    return frame.warp.forward(lon, lat)


# ----------------------------------------------------------------------------
# Fleuves
# ----------------------------------------------------------------------------


def _crop_polygon_px(grid) -> Polygon:
    lon0, lat0, lon1, lat1 = CROP_LONLAT
    t = np.linspace(0, 1, 200, endpoint=False)
    lons = np.concatenate(
        [
            lon0 + t * (lon1 - lon0),
            np.full(200, lon1),
            lon1 - t * (lon1 - lon0),
            np.full(200, lon0),
        ]
    )
    lats = np.concatenate(
        [
            np.full(200, lat0),
            lat0 + t * (lat1 - lat0),
            np.full(200, lat1),
            lat1 - t * (lat1 - lat0),
        ]
    )
    px, py = grid.lonlat_to_pixel(lons, lats)
    return Polygon(np.stack([px, py], 1))


def _chaikin(coords: np.ndarray, rounds: int = 2) -> np.ndarray:
    for _ in range(rounds):
        if len(coords) < 3:
            break
        q = 0.75 * coords[:-1] + 0.25 * coords[1:]
        r = 0.25 * coords[:-1] + 0.75 * coords[1:]
        mid = np.empty((2 * len(q), 2))
        mid[0::2], mid[1::2] = q, r
        coords = np.vstack([coords[:1], mid, coords[-1:]])
    return coords


def generalise_rivers(
    warp: Warp, count: int = 40, simplify_km: float = 4.0
) -> list[tuple[str, LineString]]:
    """≈ ``count`` grands fleuves nommés (longueur dans l'emprise), déformés et lissés."""
    geo = json.loads(real_path("rivers.geojson").read_text("utf-8"))
    crop = _crop_polygon_px(warp.grid)
    by_name: dict[str, list[LineString]] = {}
    for feat in geo["features"]:
        name = feat["properties"].get("name")
        if not name or feat["geometry"]["type"] != "LineString":
            continue
        name = RIVER_ALIASES.get(name, name)
        line = LineString(feat["geometry"]["coordinates"]).intersection(crop)
        if line.is_empty:
            continue
        parts = (
            [line]
            if line.geom_type == "LineString"
            else [g for g in line.geoms if g.geom_type == "LineString"]
        )
        by_name.setdefault(name, []).extend(parts)
    length = {n: sum(p.length for p in parts) for n, parts in by_name.items()}
    chosen = [n for n in KEY_RIVERS if n in by_name]
    for name, _ in sorted(length.items(), key=lambda kv: -kv[1]):
        if len(chosen) >= count:
            break
        if name not in chosen:
            chosen.append(name)
    out = []
    for name in chosen:
        for part in by_name[name]:
            if part.length < 25:
                continue
            xy = np.array(part.coords)
            x, y = warp.forward_px(xy[:, 0], xy[:, 1])
            line = LineString(_chaikin(np.stack([x, y], 1))).simplify(
                simplify_km / KM_PER_PX
            )
            out.append((name, line))
    return out


# ----------------------------------------------------------------------------
# Crêtes
# ----------------------------------------------------------------------------


def _skeleton_paths(skel: np.ndarray) -> list[np.ndarray]:
    """Polylignes (x, y) d'un squelette 8-connexe."""
    sk = skel.astype(bool)
    pad = np.pad(sk, 1)
    h, w = sk.shape
    offsets = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]
    degree = np.zeros(sk.shape, dtype=np.int8)
    for dy, dx in offsets:
        degree += pad[1 + dy : 1 + dy + h, 1 + dx : 1 + dx + w]
    degree = np.where(sk, degree, 0)
    nodes = sk & (degree != 2)
    visited = np.zeros_like(sk)
    paths = []

    def neighbours(y, x):
        for dy, dx in offsets:
            yy, xx = y + dy, x + dx
            if 0 <= yy < h and 0 <= xx < w and sk[yy, xx]:
                yield yy, xx

    def walk(start, first):
        path = [start, first]
        prev, cur = start, first
        while not nodes[cur] and not visited[cur]:
            visited[cur] = True
            nxt = [n for n in neighbours(*cur) if n != prev and n not in path[-3:]]
            if not nxt:
                break
            prev, cur = cur, nxt[0]
            path.append(cur)
        return path

    ys, xs = np.nonzero(nodes)
    for y, x in zip(ys, xs, strict=True):
        for n in neighbours(y, x):
            if nodes[n] and n < (y, x):
                continue
            if not visited[n] or nodes[n]:
                path = walk((y, x), n)
                if len(path) >= 2:
                    paths.append(np.array([(px, py) for py, px in path], dtype=float))
    # boucles pures (sans nœud)
    ys, xs = np.nonzero(sk & ~visited & ~nodes)
    for y, x in zip(ys, xs, strict=True):
        if visited[y, x]:
            continue
        ns = list(neighbours(y, x))
        if ns:
            visited[y, x] = True
            path = walk((y, x), ns[0])
            paths.append(np.array([(px, py) for py, px in path], dtype=float))
    return paths


def ridge_mask(
    frame: Frame,
    kind: str = "massif",
    smooth_km: float = 12.0,
    min_length_km: float = 70.0,
) -> np.ndarray:
    """Masque lissé et épuré (composantes assez grandes) d'une masse déformée."""
    mask = frame.sample(relief_masks_half()[kind].astype(np.float32), 2, order=1)
    mask = np.where(frame.inside, mask, 0.0)
    sigma = smooth_km / KM_PER_PX * frame.scale
    mask = ndimage.gaussian_filter(mask, sigma) > 0.5
    labels, n = ndimage.label(mask)
    if n:
        sizes = ndimage.sum(mask, labels, range(1, n + 1))
        min_px = (min_length_km / KM_PER_PX * frame.scale) ** 2 * 0.5
        mask = np.isin(labels, 1 + np.nonzero(sizes >= min_px)[0])
    return mask


def generalise_ridges(
    frame: Frame,
    kind: str = "massif",
    smooth_km: float = 12.0,
    min_length_km: float = 70.0,
    simplify_km: float = 6.0,
    prune_km: float = 10.0,
    mask: np.ndarray | None = None,
) -> list[LineString]:
    """Crêtes d'une masse (``massif`` ou ``hills``) : squelette du masque déformé."""
    if mask is None:
        mask = ridge_mask(frame, kind, smooth_km, min_length_km)
    skel = skeletonize(mask)
    # élagage des éperons : érosion des extrémités
    prune = int(prune_km / KM_PER_PX * frame.scale)
    for _ in range(prune):
        pad = np.pad(skel, 1)
        deg = (
            sum(
                pad[
                    1 + dy : 1 + dy + skel.shape[0], 1 + dx : 1 + dx + skel.shape[1]
                ].astype(np.int8)
                for dy in (-1, 0, 1)
                for dx in (-1, 0, 1)
            )
            - 1
        )
        skel = skel & ~(deg <= 1)
    lines = []
    for path in _skeleton_paths(skel):
        xy = frame.to_composed(path - 0.0)
        line = LineString(_chaikin(xy, 2))
        if line.length * KM_PER_PX < min_length_km:
            continue
        lines.append(line.simplify(simplify_km / KM_PER_PX))
    return lines


# ----------------------------------------------------------------------------
# Provinces
# ----------------------------------------------------------------------------


def province_frame_labels(frame: Frame, labels: np.ndarray | None = None) -> np.ndarray:
    """Indices de province par pixel de cadre (0 = mer / hors emprise), fusionnés selon ``merged_from``."""
    labels = province_labels() if labels is None else labels
    sampled = frame.sample(labels, 1, order=0).astype(np.int64)
    sampled = np.where(frame.inside, sampled, 0)
    table = merge_table()
    if table:
        # province retirée de la sélection -> 0 (hors jeu)
        index_to_id = province_index_map()
        ids = sorted({v for v in table.values()} | set(index_to_id.values()))
        new_index = {pid: i + 1 for i, pid in enumerate(ids)}
        lut = np.zeros(max(index_to_id) + 1, dtype=np.int64)
        for idx, pid in index_to_id.items():
            lut[idx] = new_index[table[pid]] if pid in table else 0
        sampled = lut[sampled]
    return sampled


def border_mask(labels: np.ndarray) -> np.ndarray:
    """Pixels de frontière entre deux provinces terrestres."""
    border = np.zeros(labels.shape, dtype=bool)
    for a, b in ((labels[:, :-1], labels[:, 1:]), (labels[:-1, :], labels[1:, :])):
        diff = (a != b) & (a > 0) & (b > 0)
        if a.shape[0] == labels.shape[0] and a.shape[1] == labels.shape[1] - 1:
            border[:, :-1] |= diff
        else:
            border[:-1, :] |= diff
    return border
