"""Relief composé de la carte composée (lot CC, C2) : ``cent-ans geo compose-relief``.

Remplace le ``heightmap.png`` provisoire de C1 (relief réel déformé) par une
hauteur **procédurale d'auteur** construite depuis ``compose/ridges.json`` :

1. base : plaines plates (rampe côtière douce, micro-relief faible), zones de
   collines et plateaux (relief réel déformé, très lissé et écrêté) ;
2. massifs et collines bâtis sur les crêtes (profil transversal en arête,
   contreforts, bruit « ridged » orienté le long de la crête) ;
3. cols : entailles nettes (axe perpendiculaire à la crête, parois raides) ;
4. plateformes : aplanissement local sous les lieux de ``selection.json`` ;
5. fleuves : lit à pente monotone posé au fond, vallée creusée autour ;
6. côtes : terre > 0, mer < 0, cohérent avec ``land_mask.png``.

Sorties dans ``data/map/`` : ``heightmap.png`` (même encodage 16 bits),
``heightmap_render.png`` (= moyenne 2x2 de la finesse, le profil RE ``boost`` est
neutralisé : le relief composé est déjà caricaturé), ``height/h_<c>_<r>.png``
(tuiles 512 du canevas complété à un multiple de 256 pixels de carte, finesse x2)
et le canal d'ombrage ``relief_shade`` (détail + occlusion).

Tout réglage est dans ``data/map/compose/relief.json``.
"""

from __future__ import annotations

import json
import time
import zlib
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage
from shapely.geometry import LineString, Point

from cent_ans_tools.compose.contract import COMPOSE_DIR, ComposedWarp
from cent_ans_tools.compose.warp import MAP_DIR
from cent_ans_tools.geo import terrain

Image.MAX_IMAGE_PIXELS = None

RELIEF_FILE = "relief.json"
RAW_DIR = Path.home() / "dev" / "cent-ans-raw" / "compose" / "c2"
#: Taille d'une tuile racine de la pyramide de relief (unités carte), cf. ``relief_pyramid.gd``.
ROOT_TILE_UNITS = 256
FINE_SCALE = 2
NOMINAL_METERS_PER_PX = 719.0
LAND_MIN_M = 0.5
SEA_MAX_M = -1.0


def load_config(path: Path | None = None) -> dict:
    """``data/map/compose/relief.json``."""
    return json.loads((path or COMPOSE_DIR / RELIEF_FILE).read_text("utf-8"))


# ----------------------------------------------------------------------------
# Outils : lissage, bruit, tracé
# ----------------------------------------------------------------------------


def smoothstep(x: np.ndarray | float) -> np.ndarray:
    """Marche douce ``3x² - 2x³`` bornée à [0, 1]."""
    x = np.clip(x, 0.0, 1.0)
    return x * x * (3.0 - 2.0 * x)


def value_noise(
    shape: tuple[int, int], wavelength: float, rng: np.random.Generator
) -> np.ndarray:
    """Bruit lisse de valeurs dans [-1, 1], longueur d'onde en pixels (bicubique)."""
    height, width = shape
    gh, gw = int(np.ceil(height / wavelength)) + 3, int(np.ceil(width / wavelength)) + 3
    grid = rng.uniform(-1.0, 1.0, (gh, gw)).astype(np.float32)
    big = Image.fromarray(grid, mode="F").resize(
        (int(round(gw * wavelength)), int(round(gh * wavelength))), Image.BICUBIC
    )
    off = int(wavelength)
    return np.asarray(big, dtype=np.float32)[off : off + height, off : off + width]


class Lattice:
    """Treillis de bruit à évaluer en coordonnées libres (enroulé)."""

    def __init__(self, rng: np.random.Generator, size: int = 256) -> None:
        """Tire un treillis aléatoire de ``size`` x ``size`` valeurs dans [-1, 1]."""
        self.values = rng.uniform(-1.0, 1.0, (size, size)).astype(np.float32)

    def sample(self, u: np.ndarray, v: np.ndarray) -> np.ndarray:
        """Bruit cubique aux coordonnées ``(u, v)`` en unités de treillis."""
        return ndimage.map_coordinates(
            self.values, [v, u], order=3, mode="grid-wrap", prefilter=False
        ).astype(np.float32)


def _window(
    points: np.ndarray, margin: float, shape: tuple[int, int]
) -> tuple[int, int, int, int]:
    """Fenêtre ``(x0, y0, x1, y1)`` bornée au canevas autour de ``points``."""
    x0 = max(int(np.floor(points[:, 0].min() - margin)), 0)
    y0 = max(int(np.floor(points[:, 1].min() - margin)), 0)
    x1 = min(int(np.ceil(points[:, 0].max() + margin)) + 1, shape[1])
    y1 = min(int(np.ceil(points[:, 1].max() + margin)) + 1, shape[0])
    return x0, y0, x1, y1


def densify(points: np.ndarray, step: float = 1.5) -> tuple[np.ndarray, np.ndarray]:
    """Polyligne rééchantillonnée tous les ``step`` px et abscisse curviligne."""
    seg = np.hypot(*np.diff(points, axis=0).T)
    cum = np.concatenate([[0.0], np.cumsum(seg)])
    n = max(int(np.ceil(cum[-1] / step)), 1)
    s = np.linspace(0.0, cum[-1], n + 1)
    return np.stack([np.interp(s, cum, points[:, 0]), np.interp(s, cum, points[:, 1])], axis=1), s


@dataclass
class Polyline:
    """Distance à une polyligne dans une fenêtre : abscisse et côté du point le plus proche."""

    dist: np.ndarray
    s: np.ndarray
    side: np.ndarray  # +1 / -1 selon le côté de la polyligne


def polyline_field(
    points: np.ndarray, window: tuple[int, int, int, int], values: np.ndarray | None = None
) -> Polyline:
    """Distance, abscisse curviligne et côté du plus proche point de la polyligne, par pixel."""
    x0, y0, x1, y1 = window
    w, h = x1 - x0, y1 - y0
    pts, s = densify(points - np.array([x0, y0]))
    if values is None:
        values = s
    img_s = Image.new("F", (w, h), -1.0)
    img_tx = Image.new("F", (w, h), 0.0)
    img_ty = Image.new("F", (w, h), 0.0)
    draws = [ImageDraw.Draw(i) for i in (img_s, img_tx, img_ty)]
    for k in range(len(pts) - 1):
        a, b = tuple(pts[k]), tuple(pts[k + 1])
        d = np.hypot(b[0] - a[0], b[1] - a[1]) or 1.0
        for draw, val in zip(
            draws, (values[k], (b[0] - a[0]) / d, (b[1] - a[1]) / d), strict=True
        ):
            draw.line([a, b], fill=float(val), width=1)
    s_img = np.asarray(img_s)
    line = s_img >= 0.0
    if not line.any():
        return Polyline(
            np.full((h, w), 1e6, np.float32),
            np.zeros((h, w), np.float32),
            np.ones((h, w), np.float32),
        )
    dist, idx = ndimage.distance_transform_edt(~line, return_indices=True)
    near_s = s_img[idx[0], idx[1]]
    tx = np.asarray(img_tx)[idx[0], idx[1]]
    ty = np.asarray(img_ty)[idx[0], idx[1]]
    yy, xx = np.mgrid[0:h, 0:w]
    cross = tx * (yy - idx[0]) - ty * (xx - idx[1])
    side = np.where(cross >= 0.0, 1.0, -1.0).astype(np.float32)
    return Polyline(dist.astype(np.float32), near_s.astype(np.float32), side)


# ----------------------------------------------------------------------------
# Données d'entrée
# ----------------------------------------------------------------------------


@dataclass
class Inputs:
    """Données d'auteur nécessaires au relief (indépendantes du disque pour les tests)."""

    land: np.ndarray  # bool (h, w)
    inside: np.ndarray  # bool (h, w), vrai dans l'emprise recadrée
    real_height: np.ndarray  # float32 (h, w), relief réel déformé (m), pour les zones
    ridges: list[dict]
    rivers: list[dict]  # {"name", "width_px", "coords"}
    places: np.ndarray  # (n, 2) positions px des lieux


def attach_free_passes(
    ridges: list[dict], passes: list[dict], max_px: float, width_px: float, depth: float = 0.7
) -> int:
    """Rattache à la crête la plus proche les cols libres (idempotent) ; renvoie le nombre ajouté."""
    attached = {p["id"] for r in ridges for p in r["passes"]}
    added = 0
    lines = [LineString(r["points"]) for r in ridges]
    for item in passes:
        if item["id"] in attached:
            continue
        pt = Point(item["at"])
        best = min(range(len(ridges)), key=lambda i: lines[i].distance(pt))
        if lines[best].distance(pt) > max_px:
            continue
        at = lines[best].interpolate(lines[best].project(pt))
        ridges[best]["passes"].append(
            {
                "id": item["id"],
                "name": item["name"],
                "at": [round(at.x, 1), round(at.y, 1)],
                "width_px": width_px,
                "depth": depth,
            }
        )
        added += 1
    return added


def load_inputs(cw: ComposedWarp, cfg: dict) -> Inputs:
    """Lit les données du disque (land_mask, ridges, rivers, lieux, relief réel déformé)."""
    from cent_ans_tools.compose import compile as composer
    from cent_ans_tools.compose import generalise as gen
    from cent_ans_tools.compose.warp import real_path

    composer.ensure_real_sources()
    land = np.asarray(Image.open(MAP_DIR / "land_mask.png")) > 0
    frame = gen.Frame(cw, 1.0)
    meta = json.loads(real_path("map.json").read_text("utf-8"))
    raw = terrain.read_png16(real_path("heightmap.png")).astype(np.float32)
    real_m = meta["height_min_m"] + raw * (meta["height_max_m"] - meta["height_min_m"]) / 65535.0
    del raw
    real_height = np.nan_to_num(frame.sample(real_m, 1, order=1), nan=0.0).astype(np.float32)
    inside = frame.inside
    doc = json.loads((COMPOSE_DIR / "ridges.json").read_text("utf-8"))
    ridges = doc["ridges"]
    free = composer._load_passes(cw)
    if attach_free_passes(
        ridges, free, cfg["passes"]["attach_free_px"], composer_pass_width()
    ):
        (COMPOSE_DIR / "ridges.json").write_text(
            json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
    rivers = []
    for feat in json.loads((MAP_DIR / "rivers.geojson").read_text("utf-8"))["features"]:
        rivers.append(
            {
                "name": feat["properties"].get("name", ""),
                "width_px": float(feat["properties"].get("width_px", 2.0)),
                "coords": np.array(feat["geometry"]["coordinates"], dtype=np.float64),
            }
        )
    selection = json.loads((COMPOSE_DIR / "selection.json").read_text("utf-8"))
    lon = np.array([s["lon"] for s in selection["settlements"]])
    lat = np.array([s["lat"] for s in selection["settlements"]])
    px, py = cw.forward(lon, lat)
    places = np.stack([np.asarray(px), np.asarray(py)], axis=1)
    return Inputs(land, inside, real_height, ridges, rivers, places)


def composer_pass_width() -> float:
    """Largeur de col par défaut (``generalise.json``)."""
    cfg = json.loads((COMPOSE_DIR / "generalise.json").read_text("utf-8"))
    return float(cfg["ridges"]["pass_width_px"])


# ----------------------------------------------------------------------------
# Étapes du relief
# ----------------------------------------------------------------------------


def build_sea(land: np.ndarray, inside: np.ndarray, cfg: dict) -> np.ndarray:
    """Profondeur marine : plateau côtier puis fonds, toujours < 0."""
    d = ndimage.distance_transform_edt(~land).astype(np.float32)
    shelf = smoothstep(d / cfg["shelf_px"])
    deep = smoothstep(d / cfg["deep_px"])
    depth = (
        1.5
        + (cfg["shelf_depth_m"] - 1.5) * shelf
        + (cfg["deep_depth_m"] - cfg["shelf_depth_m"]) * deep
    )
    depth = np.where(inside, depth, -cfg["outside_m"])
    return (-depth).astype(np.float32)


def build_base(inp: Inputs, cfg: dict, rng: np.random.Generator) -> tuple[np.ndarray, np.ndarray]:
    """Plaines plates + collines/plateaux par zones ; renvoie ``(base, distance à la côte)``."""
    land = inp.land
    shape = land.shape
    d_land = ndimage.distance_transform_edt(land).astype(np.float32)
    pcfg, zcfg = cfg["plain"], cfg["zones"]
    ramp = pcfg["coast_ramp_m"] * smoothstep(d_land / pcfg["coast_ramp_px"])
    micro = pcfg["micro_amp_m"] * value_noise(shape, pcfg["micro_wavelength_px"], rng)
    base = LAND_MIN_M + 1.0 + ramp + micro * smoothstep(d_land / 12.0)
    smooth = ndimage.gaussian_filter(np.maximum(inp.real_height, 0.0), zcfg["sigma_px"])
    x = np.maximum(smooth - zcfg["floor_m"], 0.0) / zcfg["knee_m"]
    zone = zcfg["max_m"] * np.tanh(zcfg["gain"] * x)
    zone *= 1.0 + zcfg["noise_amp"] * value_noise(shape, zcfg["noise_wavelength_px"], rng)
    base += zone * smoothstep(d_land / 6.0)
    return base.astype(np.float32), d_land


def smooth_points(points: list | np.ndarray, rounds: int = 3) -> np.ndarray:
    """Polyligne arrondie (Chaikin, extrémités fixes) : pas de sommets en pointe sur les crêtes."""
    pts = np.asarray(points, dtype=np.float64)
    for _ in range(rounds):
        if len(pts) < 3:
            break
        q = 0.75 * pts[:-1] + 0.25 * pts[1:]
        r = 0.25 * pts[:-1] + 0.75 * pts[1:]
        mid = np.empty((2 * len(q), 2))
        mid[0::2], mid[1::2] = q, r
        pts = np.vstack([pts[:1], mid, pts[-1:]])
    return pts


def _profile_params(ridge: dict, cfg: dict) -> tuple[dict, float, float]:
    """Réglages, hauteur et demi-largeur (px) d'une crête d'après son type."""
    prof = cfg["massif"] if ridge["kind"] == "massif" else cfg["hills"]
    if ridge["kind"] == "massif":
        top = np.clip(
            ridge["height_m"] * prof.get("height_gain", 1.0),
            prof["height_min_m"],
            prof["height_max_m"],
        )
    else:
        top = np.clip(
            ridge["height_m"] * prof.get("height_scale", 1.0),
            prof["height_min_m"],
            prof["height_max_m"],
        )
    half = max(ridge["width_px"] * prof["halfwidth_scale"], 4.0)
    return prof, float(top), float(half)


def ridge_height(
    ridge: dict, cfg: dict, lattice: Lattice, shape: tuple[int, int]
) -> tuple[tuple[int, int, int, int], np.ndarray]:
    """Hauteur (m) d'une crête dans sa fenêtre : profil en arête, contreforts, bruit ridged."""
    prof, top, half = _profile_params(ridge, cfg)
    pts = smooth_points(ridge["points"])
    reach = half * prof["foot_halfwidths"] * 1.15 + 4.0
    window = _window(pts, reach, shape)
    field = polyline_field(pts, window)
    t = field.dist / half
    crest = np.clip(1.0 - np.minimum(t, 1.0) ** 2, 0.0, 1.0) ** prof["crest_exponent"]
    foot = prof["foot_amp"] * np.exp(-((t / (prof["foot_halfwidths"] * 0.45)) ** 2))
    foot *= smoothstep((prof["foot_halfwidths"] - t) / 0.6)
    # Variation de la crête le long de son axe (sommets et selles).
    phase = (zlib.crc32(ridge["id"].encode()) % 997) * 7.31
    along = lattice.sample(
        field.s / prof["crest_wavelength_px"] + phase, np.full_like(field.s, phase * 0.37)
    )
    crest_mod = 1.0 + prof["crest_variation"] * along
    # Contreforts : bruit ridged étiré le long de la crête, de part et d'autre.
    near = t < prof["foot_halfwidths"]
    spur = np.zeros_like(t)
    if near.any():
        u = field.s[near] / prof["spur_wavelength_along_px"] + phase * 1.7
        v = (field.side[near] * field.dist[near]) / prof["spur_wavelength_across_px"] + phase * 0.9
        ridged = 1.0 - np.abs(lattice.sample(u, v))
        ridged = ridged**2.0
        spur[near] = ridged
    length = float(np.hypot(*np.diff(pts, axis=0).T).sum())
    ends = smoothstep(field.s / half) * smoothstep((length - field.s) / half)
    spur_weight = ends * smoothstep(t / 0.15) * (1.0 - smoothstep((t - 0.35) / (prof["foot_halfwidths"] - 0.35)))
    height = top * (crest * crest_mod + foot) + top * prof["spur_amp"] * spur * spur_weight
    # Roche : bruit ridged isotrope, présent surtout sur la masse haute.
    if prof.get("rock_amp", 0.0) > 0.0:
        x0, y0 = window[:2]
        yy, xx = np.mgrid[0 : t.shape[0], 0 : t.shape[1]].astype(np.float32)
        rock = 1.0 - np.abs(
            lattice.sample(
                (xx + x0) / prof["rock_wavelength_px"] + phase,
                (yy + y0) / prof["rock_wavelength_px"] - phase,
            )
        )
        height += top * prof["rock_amp"] * rock**2 * crest
    return window, height.astype(np.float32)


def build_ridges(
    inp: Inputs, cfg: dict, base: np.ndarray, rng: np.random.Generator
) -> np.ndarray:
    """Hauteur ajoutée par les crêtes (max sur toutes les crêtes), mètres."""
    lattice = Lattice(rng)
    out = np.zeros(base.shape, dtype=np.float32)
    for ridge in inp.ridges:
        (x0, y0, x1, y1), height = ridge_height(ridge, cfg, lattice, base.shape)
        view = out[y0:y1, x0:x1]
        np.maximum(view, height, out=view)
    return out


def pass_owners(ridges: list[dict]) -> dict[str, str]:
    """Pour chaque col, la crête la plus proche (un col n'est creusé qu'une fois)."""
    best: dict[str, tuple[float, str]] = {}
    for ridge in ridges:
        line = LineString(smooth_points(ridge["points"]))
        for p in ridge["passes"]:
            d = line.distance(Point(p["at"]))
            if p["id"] not in best or d < best[p["id"]][0]:
                best[p["id"]] = (d, ridge["id"])
    return {k: v[1] for k, v in best.items()}


def carve_passes(
    h: np.ndarray, base: np.ndarray, inp: Inputs, cfg: dict
) -> list[dict]:
    """Creuse les cols (entailles nettes, axe perpendiculaire à la crête) ; renvoie leur rapport."""
    pcfg = cfg["passes"]
    report = []
    owner = pass_owners(inp.ridges)
    for ridge in inp.ridges:
        if not ridge["passes"]:
            continue
        _, _, half = _profile_params(ridge, cfg)
        line = LineString(smooth_points(ridge["points"]))
        for p in ridge["passes"]:
            if owner[p["id"]] != ridge["id"]:
                continue
            at = np.asarray(p["at"], dtype=np.float64)
            along = line.project(Point(at))
            a = line.interpolate(max(along - 3.0, 0.0))
            b = line.interpolate(min(along + 3.0, line.length))
            tan = np.array([b.x - a.x, b.y - a.y])
            tan /= np.hypot(*tan) or 1.0
            nrm = np.array([-tan[1], tan[0]])
            wall = max(float(p.get("width_px", 6)) / 2.0, pcfg["min_half_width_px"])
            reach = half * pcfg["reach_halfwidths"]
            ext = max(reach, wall + 60.0)
            x0 = max(int(at[0] - ext), 0)
            y0 = max(int(at[1] - ext), 0)
            x1 = min(int(at[0] + ext) + 1, h.shape[1])
            y1 = min(int(at[1] + ext) + 1, h.shape[0])
            yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
            dx, dy = xx + 0.5 - at[0], yy + 0.5 - at[1]
            raw_n = dx * nrm[0] + dy * nrm[1]
            raw_t = dx * tan[0] + dy * tan[1]
            # Le défilé serpente : décalage lisse le long de l'axe de la coupe.
            raw_t = raw_t + pcfg["meander_px"] * np.sin(
                raw_n / pcfg["meander_wavelength_px"] * 2.0 * np.pi + along * 0.37
            )
            u_n = np.abs(raw_n)
            u_t = np.abs(raw_t)
            view = h[y0:y1, x0:x1]
            at_i = (int(at[1]), int(at[0]))
            here = float(h[at_i])
            floor = float(np.clip(here * (1.0 - float(p.get("depth", 0.7))), pcfg["floor_min_m"], pcfg["floor_max_m"]))
            cap = (
                floor
                - pcfg["axis_slope_m_per_px"] * u_n
                + pcfg["wall_slope_m_per_px"]
                * np.maximum(u_t - wall - pcfg["wall_widening"] * u_n, 0.0)
            )
            cap = np.maximum(cap, base[y0:y1, x0:x1])
            inside = u_n <= reach
            np.minimum(view, np.where(inside, cap, view), out=view)
            report.append({"id": p["id"], "ridge": ridge["id"], "floor_m": round(floor, 1)})
    return report


def flatten_platforms(
    h: np.ndarray, land: np.ndarray, places: np.ndarray, cfg: dict
) -> int:
    """Aplanit le terrain sous chaque lieu (moyenne locale, fondu sur ``blend_px``)."""
    pcfg = cfg["platforms"]
    r, blend = pcfg["radius_px"], pcfg["blend_px"]
    ext = int(np.ceil(r + blend)) + 1
    count = 0
    for x, y in places:
        if not np.isfinite(x) or not (0 <= x < h.shape[1] and 0 <= y < h.shape[0]):
            continue
        x0, y0 = max(int(x) - ext, 0), max(int(y) - ext, 0)
        x1, y1 = min(int(x) + ext + 1, h.shape[1]), min(int(y) + ext + 1, h.shape[0])
        yy, xx = np.mgrid[y0:y1, x0:x1].astype(np.float32)
        d = np.hypot(xx + 0.5 - x, yy + 0.5 - y)
        view, lview = h[y0:y1, x0:x1], land[y0:y1, x0:x1]
        core = (d <= r * 1.5) & lview
        if core.sum() < 3:
            continue
        target = float(view[core].mean())
        weight = (1.0 - smoothstep((d - r) / blend)) * lview
        shift = np.clip(target - view, -pcfg["max_shift_m"], pcfg["max_shift_m"])
        view += (weight * shift).astype(np.float32)
        count += 1
    return count


def orient_downstream(coords: np.ndarray, h: np.ndarray, d_land: np.ndarray) -> np.ndarray:
    """Orientation source -> embouchure : l'extrémité la plus basse (puis la plus proche de la mer)."""

    def end_height(pts: np.ndarray) -> float:
        i = np.clip(pts[:, 0].astype(int), 0, h.shape[1] - 1)
        j = np.clip(pts[:, 1].astype(int), 0, h.shape[0] - 1)
        return float(h[j, i].mean() + 0.01 * d_land[j, i].mean())

    n = max(len(coords) // 10, 2)
    return coords if end_height(coords[-n:]) <= end_height(coords[:n]) else coords[::-1]


def river_bed(
    pts: np.ndarray, ground: np.ndarray, cfg: dict
) -> tuple[np.ndarray, np.ndarray]:
    """Lit monotone (descendant vers l'embouchure) le long d'une polyligne source -> embouchure.

    Renvoie ``(échantillons densifiés, altitude du lit)``.
    """
    rcfg = cfg["rivers"]
    dense, s = densify(pts, 1.0)
    i = np.clip(dense[:, 0].astype(int), 0, ground.shape[1] - 1)
    j = np.clip(dense[:, 1].astype(int), 0, ground.shape[0] - 1)
    terrain_h = ground[j, i].astype(np.float64)
    k = int(rcfg["smooth_samples"])
    terrain_h = ndimage.uniform_filter1d(terrain_h, k, mode="nearest")
    bed = np.minimum.accumulate(terrain_h - rcfg["bed_margin_m"])
    bed = ndimage.uniform_filter1d(bed, k, mode="nearest")
    bed = np.minimum.accumulate(bed)
    floor = rcfg["bed_floor_m"] + rcfg["bed_slope_m_per_px"] * (s[-1] - s)
    bed = np.maximum(bed, floor)
    return dense, np.maximum.accumulate(bed[::-1])[::-1]


def carve_rivers(
    h: np.ndarray, land: np.ndarray, d_land: np.ndarray, inp: Inputs, cfg: dict
) -> list[dict]:
    """Pose les fleuves au fond de leur vallée (lit descendant) ; renvoie le lit de chacun."""
    rcfg = cfg["rivers"]
    beds = []
    ground = h.copy()
    for river in sorted(inp.rivers, key=lambda r: -r["width_px"]):
        pts = orient_downstream(river["coords"], ground, d_land)
        dense, bed = river_bed(pts, ground, cfg)
        valley = rcfg["valley_base_px"] + rcfg["valley_per_width_px"] * river["width_px"]
        if river["width_px"] >= rcfg["major_width_px"]:
            valley += rcfg["major_valley_px"]
        window = _window(pts, valley + 6.0, h.shape)
        field = polyline_field(dense, window)
        x0, y0, x1, y1 = window
        s_dense = np.concatenate([[0.0], np.cumsum(np.hypot(*np.diff(dense, axis=0).T))])
        bed_near = np.interp(field.s, s_dense, bed).astype(np.float32)
        view = h[y0:y1, x0:x1]
        w = smoothstep((field.dist - rcfg["channel_px"]) / valley)
        cut = np.maximum(view - bed_near, 0.0)
        vmax = rcfg["valley_depth_max_m"]
        soft = vmax * np.tanh(cut / vmax)
        cut_eff = np.where(field.dist <= rcfg["channel_px"], cut, soft)
        target = view - cut_eff * (1.0 - w)
        target = np.where(field.dist <= rcfg["channel_px"], np.minimum(view, bed_near), target)
        # Un lit ne descend jamais sous le plancher de terre.
        target = np.where(land[y0:y1, x0:x1], np.maximum(target, LAND_MIN_M + 0.1), target)
        np.minimum(view, target, out=view)
        beds.append({"name": river["name"], "dense": dense, "bed": bed})
    return beds


def build_height(inp: Inputs, cfg: dict) -> tuple[np.ndarray, dict]:
    """Hauteur composée complète (mètres, float32, taille du canevas) et rapport."""
    rng = np.random.default_rng(cfg["seed"])
    land = inp.land
    report: dict = {}
    base, d_land = build_base(inp, cfg, rng)
    ridges = build_ridges(inp, cfg, base, rng)
    h = np.where(land, base + ridges, 0.0).astype(np.float32)
    np.minimum(h, 4700.0, out=h)
    report["passes"] = carve_passes(h, base, inp, cfg)
    report["platforms"] = flatten_platforms(h, land, inp.places, cfg)
    report["rivers"] = len(carve_rivers(h, land, d_land, inp, cfg))
    h = np.where(land, np.maximum(h, LAND_MIN_M), build_sea(land, inp.inside, cfg["sea"]))
    return h.astype(np.float32), report


# ----------------------------------------------------------------------------
# Finesse x2, tuiles, ombrage, map.json
# ----------------------------------------------------------------------------


def padded_size(size_px: tuple[int, int]) -> tuple[int, int]:
    """Canevas complété à un multiple de ``ROOT_TILE_UNITS`` pixels de carte (pour les tuiles)."""
    w, h = size_px
    unit = ROOT_TILE_UNITS
    return -(-w // unit) * unit, -(-h // unit) * unit


def upsample2(array: np.ndarray) -> np.ndarray:
    """Surcouche bilinéaire exacte x2 (ce que renvoie un échantillonnage GPU aux centres fins)."""

    def along_axis(a: np.ndarray, axis: int) -> np.ndarray:
        prev = np.take(a, np.r_[0, np.arange(a.shape[axis] - 1)], axis=axis)
        nxt = np.take(a, np.r_[np.arange(1, a.shape[axis]), a.shape[axis] - 1], axis=axis)
        shape = list(a.shape)
        shape[axis] *= 2
        out = np.empty(shape, dtype=np.float32)
        low = [slice(None)] * a.ndim
        high = [slice(None)] * a.ndim
        low[axis], high[axis] = slice(0, None, 2), slice(1, None, 2)
        out[tuple(low)] = 0.75 * a + 0.25 * prev
        out[tuple(high)] = 0.75 * a + 0.25 * nxt
        return out

    return along_axis(along_axis(array.astype(np.float32), 0), 1)


def pad_canvas(h: np.ndarray, size: tuple[int, int]) -> np.ndarray:
    """Complète ``h`` à ``size = (w, h)`` en prolongeant les bords."""
    pad_w, pad_h = size[0] - h.shape[1], size[1] - h.shape[0]
    return np.pad(h, ((0, pad_h), (0, pad_w)), mode="edge")


def fine_height(h_padded: np.ndarray, cfg: dict) -> np.ndarray:
    """Hauteur fine (x2) : surcouche bilinéaire + détail rocheux/terreux de la finesse."""
    fcfg = cfg["fine"]
    rng = np.random.default_rng(cfg["seed"] + 7)
    fine = upsample2(h_padded)
    shape = fine.shape
    weights = np.array([0.5, 0.33, 0.17][: len(fcfg["wavelength_fine_px"])])
    detail = np.zeros(shape, dtype=np.float32)
    for wave, weight in zip(fcfg["wavelength_fine_px"], weights, strict=False):
        detail += weight * value_noise(shape, float(wave), rng)
    amp = fcfg["detail_floor_m"] + (fcfg["detail_amp_m"] - fcfg["detail_floor_m"]) * smoothstep(
        np.maximum(fine, 0.0) / fcfg["detail_ref_m"]
    )
    fine += detail * amp.astype(np.float32)
    del detail, amp
    return fine


def write_outputs(h: np.ndarray, cw: ComposedWarp, cfg: dict, map_dir: Path = MAP_DIR) -> dict:
    """Écrit heightmap, heightmap_render, tuiles ``height/``, ombrage BC5 et met ``map.json`` à jour."""
    from cent_ans_tools.geo import block_compress, relief, relief_shade

    size = (h.shape[1], h.shape[0])
    enc = terrain.height_to_uint16(h)
    terrain.write_png16(enc, map_dir / "heightmap.png")
    original = terrain.uint16_to_height(enc).astype(np.float32)
    padded = pad_canvas(original, padded_size(size))
    fine = fine_height(padded, cfg)
    land = relief_shade.upsample_sign(padded, FINE_SCALE)
    fine = relief_shade.enforce_coast(fine, land)
    encoded = terrain.height_to_uint16(fine)
    tiles = relief.write_tiles(encoded, map_dir / relief.TILE_DIR)
    fine = terrain.uint16_to_height(encoded).astype(np.float32)
    del encoded
    render_full = relief_shade.block_mean(fine, FINE_SCALE)
    render_full = np.where(padded > 0.0, render_full, padded)
    render = render_full[: size[1], : size[0]]
    terrain.write_png16(terrain.height_to_uint16(render), map_dir / relief_shade.RENDER_HEIGHTMAP)
    detail = fine - relief_shade.bilinear_upsample(render_full, FINE_SCALE)
    occ = relief_shade.occlusion(fine, NOMINAL_METERS_PER_PX / FINE_SCALE)
    detail[~land] = 0.0
    occ[~land] = 0.0
    shade = relief_shade.encode_shade(detail, occ)
    del detail, occ, fine
    for stale in map_dir.glob(relief_shade.RELIEF_SHADE_STEM + "_[0-9]*.png"):
        stale.unlink()
    shade_meta = block_compress.write_relief_bc5(shade, map_dir).meta
    meta_path = map_dir / "map.json"
    meta = json.loads(meta_path.read_text("utf-8"))
    pw, ph = padded_size(size)
    meta["height_tiles"] = {
        "size_px": [pw * FINE_SCALE, ph * FINE_SCALE],
        "tile_px": relief.TILE_PX,
        "dir": relief.TILE_DIR,
        "pattern": relief.TILE_PATTERN,
    }
    meta["render_heightmap"] = {
        "file": relief_shade.RENDER_HEIGHTMAP,
        "composed": "relief composé (CC C2) : déjà caricaturé, aucun relevé de rendu (boost RE neutralisé)",
        "boost": None,
    }
    meta["relief_shade"] = {
        "detail_scale_m": relief_shade.DETAIL_SCALE_M,
        "bc5": shade_meta,
    }
    meta.setdefault("sources", {})["dem"] = {
        "name": "relief procédural d'auteur (CC C2) sur compose/ridges.json",
        "settings": "compose/relief.json",
    }
    meta_path.write_text(json.dumps(meta, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
    return {"tiles": len(tiles), "fine_size": [pw * FINE_SCALE, ph * FINE_SCALE]}


# ----------------------------------------------------------------------------
# Orchestration
# ----------------------------------------------------------------------------


def compose_relief(
    cfg_path: Path | None = None,
    map_dir: Path = MAP_DIR,
    write: bool = True,
    board: Path | None = None,
    progress: bool = True,
) -> dict:
    """Construit le relief composé ; renvoie le rapport."""
    t0 = time.time()

    def log(msg: str) -> None:
        if progress:
            print(f"[relief] {msg} ({time.time() - t0:.0f} s)", flush=True)

    cfg = load_config(cfg_path)
    cw = ComposedWarp.load(COMPOSE_DIR)
    inp = load_inputs(cw, cfg)
    log("entrées lues")
    h, report = build_height(inp, cfg)
    report["passes"] = len(report["passes"])
    log(f"hauteur {float(h.min()):.0f}..{float(h.max()):.0f} m")
    if write:
        report["outputs"] = write_outputs(h, cw, cfg, map_dir)
        log("sorties écrites")
    if board is not None:
        report["board"] = str(make_board(h, board))
        log("planche")
    report["seconds"] = round(time.time() - t0, 1)
    return report


def hillshade(h: np.ndarray, width: int, window: tuple[int, int, int, int] | None = None) -> Image.Image:
    """Planche ombrée 2D : hypsométrie douce + ombrage exagéré (contrôle visuel hors dépôt)."""
    x0, y0, x1, y1 = window or (0, 0, h.shape[1], h.shape[0])
    sub = h[y0:y1, x0:x1]
    scale = width / sub.shape[1]
    small = np.asarray(
        Image.fromarray(np.ascontiguousarray(sub)).resize(
            (width, int(sub.shape[0] * scale)), Image.BILINEAR
        ),
        dtype=np.float32,
    )
    gy, gx = np.gradient(small)
    z = 6.0 / 719.0 * scale
    nx, ny = -gx * z, -gy * z
    light = np.array([-0.6, -0.6, 0.55])
    norm = np.sqrt(nx**2 + ny**2 + 1.0)
    shade = np.clip((nx * light[0] + ny * light[1] + light[2]) / norm / 0.6, 0.0, 1.0)
    t = np.clip(small / 3000.0, 0.0, 1.0)[..., None]
    low, mid, high = (np.array(c) for c in ((0.62, 0.72, 0.45), (0.72, 0.62, 0.40), (0.92, 0.90, 0.88)))
    color = np.where(t < 0.4, low + (mid - low) * (t / 0.4), mid + (high - mid) * np.clip((t - 0.4) / 0.6, 0, 1))
    ground = color * (0.35 + 0.65 * shade[..., None])
    sea = np.array([0.55, 0.72, 0.88]) * (1.0 - 0.4 * np.clip(-small / 150.0, 0, 1)[..., None])
    rgb = np.where((small > 0)[..., None], ground, sea)
    return Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8))


def make_board(h: np.ndarray, out_dir: Path, width: int = 1800) -> Path:
    """Planche 2D ombrée de la carte entière + un détail des Alpes."""
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / "planche_c2.png"
    hillshade(h, width).save(path)
    hillshade(h, 1400, (2200, 3000, 3900, 4400)).save(out_dir / "planche_c2_alpes.png")
    return path
