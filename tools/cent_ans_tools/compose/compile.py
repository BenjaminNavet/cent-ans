"""Compilateur de la carte composée (lot CC, C1) : ``cent-ans geo compose``.

Produit, à la place de l'étape 0 réelle (ETOPO / Natural Earth), le contrat que
lit toute la chaîne aval :

- ``compose/warp.json`` + ``warp_forward.f32`` + ``warp_inverse.f32`` (déformation) ;
- ``land_mask.png``, ``coastline.geojson`` (côtes généralisées, pixels composés) ;
- ``rivers.geojson`` (≈ 40 grands fleuves continus) ;
- ``heightmap.png`` PROVISOIRE (relief réel rééchantillonné ; C2 le remplace) ;
- ``compose/ridges.json`` (crêtes d'auteur, entrée de C2) ;
- ``map.json`` (``"crs": "composed"``).

Les sources réelles (masque, relief, fleuves de l'ancienne carte) sont lues dans
``data/map/real/`` (hors git, reconstruit depuis l'historique par
:func:`ensure_real_sources`).
"""

from __future__ import annotations

import json
import shutil
import subprocess
import time
from datetime import UTC, datetime
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage
from shapely.affinity import scale as affine_scale
from shapely.geometry import LineString, MultiPolygon, Point, Polygon, shape
from shapely.ops import linemerge, unary_union

from cent_ans_tools.compose import generalise as gen
from cent_ans_tools.compose import lakes_fine
from cent_ans_tools.compose.contract import COMPOSE_DIR, ComposedWarp
from cent_ans_tools.compose.warp import (
    CROP_LONLAT,
    KM_PER_PX,
    MAP_DIR,
    REAL_DIR,
    Warp,
    build_warp,
    real_path,
)
from cent_ans_tools.geo import terrain, vectors
from cent_ans_tools.geo.project import MapGrid
from cent_ans_tools.paths import REPO_DIR

Image.MAX_IMAGE_PIXELS = None

#: Commit qui contient la carte réelle (avant tout fichier composé).
REAL_SOURCE_REV = "8e251a874"
REAL_FILES = (
    "map.json",
    "land_mask.png",
    "heightmap.png",
    "rivers.geojson",
    "coastline.geojson",
    "province_ids.png",
    "provinces.geojson",
)
#: Échelle nominale de la carte composée (réglable en C6), mètres par pixel.
NOMINAL_METERS_PER_PX = 719.0
#: Plancher de hauteur d'une terre émergée et plafond de l'eau dans la hauteur provisoire.
LAND_FLOOR_M = 0.5
WATER_CEIL_M = -1.0
OUTSIDE_M = -50.0


# ----------------------------------------------------------------------------
# Sources réelles
# ----------------------------------------------------------------------------


def ensure_real_sources() -> Path:
    """Garantit ``data/map/real/`` : copie de la carte réelle courante ou ``git show``."""
    REAL_DIR.mkdir(parents=True, exist_ok=True)
    (REAL_DIR / ".gitignore").write_text("*\n!.gitignore\n", encoding="utf-8")
    current = json.loads((MAP_DIR / "map.json").read_text("utf-8"))
    for name in REAL_FILES:
        target = REAL_DIR / name
        if target.exists():
            continue
        if current.get("crs") != "composed" and (MAP_DIR / name).exists():
            shutil.copyfile(MAP_DIR / name, target)
        else:
            blob = subprocess.run(
                ["git", "show", f"{REAL_SOURCE_REV}:data/map/{name}"],
                cwd=REPO_DIR,
                check=True,
                capture_output=True,
            ).stdout
            target.write_bytes(blob)
    return REAL_DIR


def load_config() -> dict:
    """``data/map/compose/generalise.json``."""
    return json.loads((COMPOSE_DIR / "generalise.json").read_text("utf-8"))


# ----------------------------------------------------------------------------
# Déformation (+ épingles d'auteur)
# ----------------------------------------------------------------------------


def apply_pins(warp: Warp, pins: list[dict]) -> int:
    """Corrige ``warp.fwd`` pour que chaque épingle tombe sur sa cible, puis recalcule l'inverse."""
    if not pins:
        return 0
    from cent_ans_tools.compose.warp import _inverse_field

    f = warp.factor
    ny, nx = warp.fwd.shape[1:]
    cols = (np.arange(nx) + 0.5) * f + warp.box[0]
    rows = (np.arange(ny) + 0.5) * f + warp.box[1]
    gx, gy = np.meshgrid(cols, rows)
    shift = np.zeros_like(warp.fwd)
    for pin in pins:
        px, py = warp.grid.lonlat_to_pixel(*pin["lonlat"])
        cx, cy = warp.forward_px(px, py)
        dx, dy = pin["target_px"][0] - float(cx), pin["target_px"][1] - float(cy)
        radius = pin.get("radius_km", 150.0) / KM_PER_PX
        weight = np.exp(-((gx - px) ** 2 + (gy - py) ** 2) / (2.0 * radius**2))
        shift[0] += weight * dx / f
        shift[1] += weight * dy / f
    warp.fwd = warp.fwd + shift
    warp.inv = _inverse_field(warp.fwd)
    return len(pins)


def make_contract(progress: bool = False, reuse: bool = True) -> ComposedWarp:
    """Construit (ou relit) la déformation du contrat, épingles d'auteur appliquées."""
    if reuse and (COMPOSE_DIR / "warp.json").exists():
        return ComposedWarp.load(COMPOSE_DIR)
    warp = build_warp(progress=progress)
    pins = json.loads((COMPOSE_DIR / "pins.json").read_text("utf-8"))["pins"]
    warp.params["pins"] = apply_pins(warp, pins)
    contract = ComposedWarp.from_warp(warp)
    contract.refine_inverse()
    contract.save(COMPOSE_DIR)
    return contract


# ----------------------------------------------------------------------------
# Terre, îles, côtes
# ----------------------------------------------------------------------------


def _chaikin_ring(coords: np.ndarray, rounds: int) -> np.ndarray:
    """Chaikin sur un anneau fermé (premier point répété en dernier)."""
    pts = coords[:-1]
    for _ in range(rounds):
        nxt = np.roll(pts, -1, axis=0)
        q, r = 0.75 * pts + 0.25 * nxt, 0.25 * pts + 0.75 * nxt
        out = np.empty((2 * len(pts), 2))
        out[0::2], out[1::2] = q, r
        pts = out
    return np.vstack([pts, pts[:1]])


def _smooth_polygon(poly: Polygon, simplify_px: float, rounds: int) -> Polygon | None:
    def ring(r) -> np.ndarray | None:
        line = LineString(r.coords).simplify(simplify_px)
        arr = np.array(line.coords)
        if len(arr) < 4:
            return None
        return _chaikin_ring(arr, rounds)

    ext = ring(poly.exterior)
    if ext is None:
        return None
    holes = [h for h in (ring(i) for i in poly.interiors) if h is not None]
    out = Polygon(ext, holes).buffer(0)
    if out.is_empty:
        return None
    if isinstance(out, MultiPolygon):
        out = max(out.geoms, key=lambda g: g.area)
    return out


def carve_straits(land: np.ndarray, cw: ComposedWarp, straits: list[dict]) -> None:
    """Rend de l'eau le long des détroits d'auteur (largeur en km nominaux)."""
    from PIL import ImageDraw

    img = Image.new("L", (land.shape[1], land.shape[0]), 0)
    draw = ImageDraw.Draw(img)
    for strait in straits:
        lon, lat = np.array(strait["path"]).T
        x, y = cw.forward(lon, lat)
        width = max(1, int(round(strait["width_km"] / KM_PER_PX)))
        draw.line(list(zip(x.tolist(), y.tolist(), strict=True)), fill=255, width=width)
    land[np.asarray(img) > 0] = False


def _polygonize(mask: np.ndarray) -> list[Polygon]:
    from rasterio import features

    out = []
    for geom, value in features.shapes(mask.astype(np.uint8), mask=mask):
        if value:
            out.append(shape(geom))
    return out


def real_island_polygon(
    real_land: np.ndarray, cw: ComposedWarp, lonlat, window: int = 150
) -> Polygon | None:
    """Silhouette réelle d'une île (composante la plus proche), déformée, en px composés."""
    px, py = (float(v) for v in cw.grid.lonlat_to_pixel(*lonlat))
    x0, y0 = max(int(px) - window, 0), max(int(py) - window, 0)
    x1 = min(int(px) + window, real_land.shape[1])
    y1 = min(int(py) + window, real_land.shape[0])
    sub = real_land[y0:y1, x0:x1] > 127
    if not sub.any():
        return None
    labels, _ = ndimage.label(sub)
    dist, (ri, ci) = ndimage.distance_transform_edt(~sub, return_indices=True)
    cy, cx = int(py) - y0, int(px) - x0
    cy, cx = min(max(cy, 0), sub.shape[0] - 1), min(max(cx, 0), sub.shape[1] - 1)
    if dist[cy, cx] > 40:
        return None
    label = labels[ri[cy, cx], ci[cy, cx]]
    comp = labels == label
    ys, xs = np.nonzero(comp)
    if ys.min() == 0 or xs.min() == 0 or ys.max() == sub.shape[0] - 1:
        return None  # touche le bord de la fenêtre : trop grande pour ce repli
    if xs.max() == sub.shape[1] - 1:
        return None
    soft = ndimage.gaussian_filter(comp.astype(np.float32), 1.2) > 0.5
    polys = _polygonize(soft)
    if not polys:
        return None
    poly = max(polys, key=lambda p: p.area)
    ext = np.array(poly.exterior.coords) + (x0, y0)
    cxy = cw.forward_px(ext[:, 0], ext[:, 1])
    return Polygon(np.stack(cxy, axis=1)).buffer(0)


def build_land(
    frame: gen.Frame, cw: ComposedWarp, cfg: dict
) -> tuple[MultiPolygon, np.ndarray, dict]:
    """Terre généralisée : polygones lissés, masque 0/255 cohérent, statistiques."""
    land_cfg = cfg["land"]
    to_km2 = KM_PER_PX**2
    land = frame.sample(gen.land_half(), 2, order=1)
    land = np.where(frame.inside, land, 0.0)
    sigma = land_cfg["smooth_km"] / KM_PER_PX * frame.scale
    mask = ndimage.gaussian_filter(land, sigma) > 0.5
    del land
    carve_straits(mask, cw, cfg["straits"])

    keep = cfg["islands"]["keep"]
    keep_xy = [tuple(float(v) for v in cw.forward(*item["lonlat"])) for item in keep]
    # petites composantes de terre : supprimées sauf si une île gardée y tombe
    labels, n = ndimage.label(mask)
    sizes = ndimage.sum(mask, labels, range(1, n + 1)) * to_km2
    protected = set()
    for x, y in keep_xy:
        xi, yi = int(round(x)), int(round(y))
        r = 25
        win = labels[max(yi - r, 0) : yi + r, max(xi - r, 0) : xi + r]
        protected.update(int(v) for v in np.unique(win) if v)
    keep_labels = [
        i + 1
        for i, size in enumerate(sizes)
        if size >= land_cfg["min_island_km2"] or (i + 1) in protected
    ]
    mask = np.isin(labels, keep_labels)
    # petits plans d'eau intérieurs : comblés
    wl, wn = ndimage.label(~mask)
    wsizes = ndimage.sum(~mask, wl, range(1, wn + 1)) * to_km2
    border = set(np.unique(np.concatenate([wl[0], wl[-1], wl[:, 0], wl[:, -1]])))
    fill = [
        i + 1
        for i, size in enumerate(wsizes)
        if size < land_cfg["min_lake_km2"] and (i + 1) not in border
    ]
    mask |= np.isin(wl, fill)

    simplify_px = land_cfg["simplify_km"] / KM_PER_PX
    rounds = int(land_cfg.get("ring_smooth_rounds", 2))
    polys = []
    for poly in _polygonize(mask):
        smooth = _smooth_polygon(poly, simplify_px, rounds)
        if smooth is not None:
            polys.append(smooth)

    # îles gardées : silhouette conservée, agrandie par homothétie si trop petite
    real_land = np.asarray(Image.open(real_path("land_mask.png")))
    stats: dict = {"islands": {}}
    extra: list[Polygon] = []
    min_kept = land_cfg["min_kept_island_km2"]
    for item, (x, y) in zip(keep, keep_xy, strict=True):
        target = Point(x, y)
        hit = next((p for p in polys if p.distance(target) < 12), None)
        status = "conservée"
        if hit is None:
            hit = real_island_polygon(real_land, cw, item["lonlat"])
            status = "silhouette réelle"
            if hit is None or hit.is_empty:
                stats["islands"][item["name"]] = "absente"
                continue
        area = hit.area * to_km2
        if area < min_kept:
            factor = min(
                np.sqrt(min_kept / max(area, 1e-6)), land_cfg["max_island_scale"]
            )
            grown = affine_scale(hit, factor, factor, origin="centroid")
            if status == "conservée":
                polys.remove(hit)
            extra.append(grown)
            status += f" x{factor:.2f}"
        elif status != "conservée":
            extra.append(hit)
        stats["islands"][item["name"]] = status
    merged = unary_union([p.buffer(0) for p in polys + extra])
    geoms = [merged] if isinstance(merged, Polygon) else list(merged.geoms)
    final = MultiPolygon([g for g in geoms if isinstance(g, Polygon)])
    # recoupe sur le canevas et sur l'emprise
    w, h = cw.size_px
    final = final.intersection(Polygon([(0, 0), (w, 0), (w, h), (0, h)]))
    geoms = [final] if isinstance(final, Polygon) else list(final.geoms)
    final = MultiPolygon([g for g in geoms if isinstance(g, Polygon)])
    from rasterio import features

    raster = features.rasterize(
        [(final, 255)], out_shape=(h, w), fill=0, dtype=np.uint8
    )
    raster[~frame.inside] = 0
    stats["polygons"] = len(final.geoms)
    stats["land_fraction"] = float((raster > 0).mean())
    return final, raster, stats


def coastline_frame(land: MultiPolygon):  # noqa: ANN201
    """Côtes (anneaux extérieurs et intérieurs) en GeoDataFrame pixelisé."""
    import geopandas as gpd

    lines = []
    for poly in land.geoms:
        lines.append(LineString(np.round(np.array(poly.exterior.coords), 1)))
        lines.extend(
            LineString(np.round(np.array(r.coords), 1)) for r in poly.interiors
        )
    frame = gpd.GeoDataFrame(
        {"scalerank": [0] * len(lines), "featurecla": ["Coastline"] * len(lines)},
        geometry=lines,
        crs=None,
    )
    return frame


# ----------------------------------------------------------------------------
# Fleuves
# ----------------------------------------------------------------------------


def _chain(parts: list[LineString], tol: float) -> LineString | None:
    """Chaîne les tronçons d'un même fleuve en une polyligne continue (la plus longue)."""
    merged = linemerge(parts)
    pieces = [merged] if merged.geom_type == "LineString" else list(merged.geoms)
    pieces.sort(key=lambda p: -p.length)
    coords = [np.array(pieces[0].coords)]
    rest = pieces[1:]
    changed = True
    while rest and changed:
        changed = False
        for piece in list(rest):
            arr = np.array(piece.coords)
            chain = coords[0]
            options = (
                (np.hypot(*(chain[-1] - arr[0])), "append", arr),
                (np.hypot(*(chain[-1] - arr[-1])), "append", arr[::-1]),
                (np.hypot(*(chain[0] - arr[-1])), "prepend", arr),
                (np.hypot(*(chain[0] - arr[0])), "prepend", arr[::-1]),
            )
            dist, mode, oriented = min(options, key=lambda o: o[0])
            if dist > tol:
                continue
            coords[0] = (
                np.vstack([chain, oriented])
                if mode == "append"
                else np.vstack([oriented, chain])
            )
            rest.remove(piece)
            changed = True
    return LineString(coords[0]) if len(coords[0]) >= 2 else None


def _orient_like_parts(chain: LineString, parts: list[LineString]) -> LineString:
    """Sens du fleuve (source -> embouchure) d'après la majorité des tronçons d'origine."""
    c = np.array(chain.coords)
    axis = c[-1] - c[0]
    score = 0.0
    for part in parts:
        p = np.array(part.coords)
        score += float(np.dot(p[-1] - p[0], axis))
    return LineString(c[::-1]) if score < 0 else chain


def build_rivers(
    cw: ComposedWarp, cfg: dict, land_mask: np.ndarray
) -> tuple[list[dict], dict]:
    """Grands fleuves continus en px composés : liste de features GeoJSON et statistiques."""
    rcfg = cfg["rivers"]
    aliases = rcfg.get("aliases", {})
    geo = json.loads(real_path("rivers.geojson").read_text("utf-8"))
    crop = gen._crop_polygon_px(cw.grid)
    by_name: dict[str, list[LineString]] = {}
    rank: dict[str, int] = {}
    for feat in geo["features"]:
        props = feat["properties"]
        name = props.get("name")
        if not name or props.get("featurecla") != "River":
            continue
        name = aliases.get(name, name)
        line = LineString(feat["geometry"]["coordinates"]).intersection(crop)
        parts = (
            [line]
            if line.geom_type == "LineString"
            else [g for g in getattr(line, "geoms", []) if g.geom_type == "LineString"]
        )
        parts = [p for p in parts if not p.is_empty]
        if parts:
            by_name.setdefault(name, []).extend(parts)
            rank[name] = min(rank.get(name, 99), int(props.get("scalerank") or 10))
    chains: dict[str, LineString] = {}
    for name, parts in by_name.items():
        chain = _chain(parts, rcfg["join_tolerance_px"])
        if chain is not None:
            chains[name] = _orient_like_parts(chain, parts)
    min_len = rcfg.get("min_length_km", 150) / KM_PER_PX
    chosen = [n for n in rcfg["must_keep"] if n in chains]
    for name, line in sorted(chains.items(), key=lambda kv: -kv[1].length):
        if len(chosen) >= rcfg["count"]:
            break
        if name not in chosen and line.length >= min_len:
            chosen.append(name)

    water_dist, (wi, wj) = ndimage.distance_transform_edt(
        land_mask > 0, return_indices=True
    )
    h, w = land_mask.shape
    features = []
    for name in chosen:
        real_line = chains[name]
        dense = real_line.segmentize(6.0)
        xy = np.array(dense.coords)
        x, y = cw.forward_px(xy[:, 0], xy[:, 1])
        pts = np.stack([x, y], axis=1)
        pts = pts[np.isfinite(pts).all(axis=1)]
        if len(pts) < 2:
            continue
        line = LineString(pts).simplify(rcfg["simplify_px"])
        pts = gen._chaikin(np.array(line.coords), rcfg["chaikin_rounds"])
        pts = _snap_mouth(pts, land_mask, water_dist, wi, wj, rcfg["mouth_snap_px"])
        pts = np.clip(pts, [0, 0], [w - 1, h - 1])
        scalerank = rank[name]
        features.append(
            {
                "name": name,
                "scalerank": scalerank,
                "featurecla": "River",
                "source": "composed",
                "width_px": round(
                    float(np.clip(1.0 + (12 - scalerank) * 0.4, 1.2, 4.6)), 2
                ),
                "length_km": round(LineString(pts).length * KM_PER_PX, 1),
                "coords": np.round(pts, 1),
            }
        )
    return features, {"chosen": chosen, "candidates": len(chains)}


def _snap_mouth(
    pts: np.ndarray,
    land: np.ndarray,
    dist: np.ndarray,
    wi: np.ndarray,
    wj: np.ndarray,
    max_px: float,
) -> np.ndarray:
    """Coupe l'extrémité aval dans la mer et la prolonge jusqu'à l'eau la plus proche."""
    h, w = land.shape

    def is_land(p) -> bool:
        x, y = int(np.clip(p[0], 0, w - 1)), int(np.clip(p[1], 0, h - 1))
        return bool(land[y, x] > 0)

    keep = len(pts)
    while keep > 2 and not is_land(pts[keep - 1]):
        keep -= 1
    out = pts[:keep]
    last = out[-1]
    x, y = int(np.clip(last[0], 0, w - 1)), int(np.clip(last[1], 0, h - 1))
    if land[y, x] > 0 and 0 < dist[y, x] <= max_px:
        # pixel de terre le plus proche de la mer : on cherche l'eau la plus proche
        near = np.array([wj[y, x], wi[y, x]], dtype=float)  # (x, y) du pixel d'eau
        out = np.vstack([out, near + 0.5])
    elif keep < len(pts):
        out = np.vstack([out, pts[keep]])
    return out


# ----------------------------------------------------------------------------
# Relief provisoire
# ----------------------------------------------------------------------------


def provisional_heightmap(frame: gen.Frame, land_mask: np.ndarray) -> np.ndarray:
    """Relief réel rééchantillonné par la déformation inverse (mètres), cohérent avec la côte."""
    meta = json.loads(real_path("map.json").read_text("utf-8"))
    lo, hi = meta["height_min_m"], meta["height_max_m"]
    raw = terrain.read_png16(real_path("heightmap.png")).astype(np.float32)
    real_m = lo + raw * (hi - lo) / 65535.0
    height = frame.sample(real_m, 1, order=1)
    land = land_mask > 0
    height = np.where(
        land, np.maximum(height, LAND_FLOOR_M), np.minimum(height, WATER_CEIL_M)
    )
    height = np.where(frame.inside, height, OUTSIDE_M)
    return height.astype(np.float32)


# ----------------------------------------------------------------------------
# Crêtes
# ----------------------------------------------------------------------------


def _load_passes(cw: ComposedWarp) -> list[dict]:
    items = json.loads((MAP_DIR / "crossings.json").read_text("utf-8"))["crossings"]
    out = []
    for item in items:
        if item.get("type") != "pass":
            continue
        x, y = cw.forward(*item["lonlat"])
        out.append({"id": item["id"], "name": item["name"], "at": (float(x), float(y))})
    return out


def build_ridges(cw: ComposedWarp, cfg: dict, real_m: np.ndarray) -> tuple[dict, dict]:
    """Crêtes d'auteur (``ridges.json``) : polylignes, largeur (masque), hauteur (relief réel), cols."""
    rcfg = cfg["ridges"]
    frame = gen.Frame(cw, rcfg["scale"])
    passes = _load_passes(cw)
    local_max = ndimage.maximum_filter(real_m, size=7)
    ridges = []
    for kind in ("massif", "hills"):
        mask = gen.ridge_mask(frame, kind, rcfg["smooth_km"], rcfg["min_length_km"])
        lines = gen.generalise_ridges(
            frame,
            kind,
            rcfg["smooth_km"],
            rcfg["min_length_km"],
            rcfg["simplify_km"],
            rcfg["prune_km"],
            mask=mask,
        )
        edt = ndimage.distance_transform_edt(mask)
        for number, line in enumerate(lines, start=1):
            pts = np.array(line.coords)
            fi = np.clip((pts[:, 0] * frame.scale).astype(int), 0, edt.shape[1] - 1)
            fj = np.clip((pts[:, 1] * frame.scale).astype(int), 0, edt.shape[0] - 1)
            width = float(np.median(2.0 * edt[fj, fi])) / frame.scale
            dense = np.array(line.segmentize(10.0).coords)
            rx, ry = cw.inverse_px(dense[:, 0], dense[:, 1])
            ok = np.isfinite(rx)
            ri = np.clip(rx[ok].astype(int), 0, real_m.shape[1] - 1)
            rj = np.clip(ry[ok].astype(int), 0, real_m.shape[0] - 1)
            height = float(np.percentile(local_max[rj, ri], 90)) if ok.any() else 0.0
            ridge_passes = []
            for p in passes:
                d = line.distance(Point(p["at"]))
                if d <= rcfg["pass_attach_px"]:
                    along = line.project(Point(p["at"]))
                    at = line.interpolate(along)
                    ridge_passes.append(
                        {
                            "id": p["id"],
                            "name": p["name"],
                            "at": [round(at.x, 1), round(at.y, 1)],
                            "width_px": rcfg["pass_width_px"],
                            "depth": 0.7,
                        }
                    )
            ridges.append(
                {
                    "id": f"{kind}_{number:02d}",
                    "kind": kind,
                    "points": np.round(pts, 1).tolist(),
                    "width_px": round(max(width, 2.0), 1),
                    "height_m": float(round(height / 50.0) * 50),
                    "passes": ridge_passes,
                }
            )
    attached = {p["id"] for r in ridges for p in r["passes"]}
    doc = {
        "version": 1,
        "description": (
            "Crêtes d'auteur de la carte composée (lot CC C1) : polylignes en pixels du canevas "
            "composé, largeur de la masse, hauteur réelle (m), cols attachés. Entrée de C2."
        ),
        "size_px": list(cw.size_px),
        "ridges": ridges,
    }
    return doc, {
        "ridges": len(ridges),
        "passes_total": len(passes),
        "passes_attached": len(attached),
        "passes_free": sorted({p["id"] for p in passes} - attached),
    }


def keep_authored_ridges(doc: dict, path: Path | None = None) -> int:
    """Reporte dans ``doc`` (crêtes générées) les retouches d'auteur de ``ridges.json`` existant.

    Les crêtes ``authored`` (éditeur CE, chaînes de ``ridges_author``) sont conservées telles
    quelles ; une crête générée de même ``id`` ou listée dans ``deleted_ids`` est écartée.
    Renvoie le nombre de crêtes d'auteur conservées.
    """
    path = path or COMPOSE_DIR / "ridges.json"
    if not path.exists():
        return 0
    old = json.loads(path.read_text("utf-8"))
    authored = [r for r in old["ridges"] if r.get("authored")]
    skip = {r["id"] for r in authored} | set(old.get("deleted_ids", []))
    doc["ridges"] = [r for r in doc["ridges"] if r["id"] not in skip] + authored
    for key in ("deleted_ids", "chains_seeded"):
        if key in old:
            doc[key] = old[key]
    return len(authored)


# ----------------------------------------------------------------------------
# map.json
# ----------------------------------------------------------------------------


def composed_map_json(cw: ComposedWarp) -> dict:
    """``map.json`` du CRS composé (contrat commun)."""
    real = json.loads(real_path("map.json").read_text("utf-8"))
    rg = cw.real_grid
    from cent_ans_tools.geo import relief

    fine = MapGrid(
        (0, 0, 1, 1),
        cw.size_px[0] * relief.FINE_SCALE,
        cw.size_px[1] * relief.FINE_SCALE,
    )
    return {
        "crs": "composed",
        "size_px": list(cw.size_px),
        "meters_per_px": NOMINAL_METERS_PER_PX,
        "height_min_m": real["height_min_m"],
        "height_max_m": real["height_max_m"],
        "extent_lonlat": list(CROP_LONLAT),
        "composed": {
            "warp": "compose/warp.json",
            "size_px": list(cw.size_px),
            "meters_per_px": NOMINAL_METERS_PER_PX,
            "crop_lonlat": list(CROP_LONLAT),
        },
        "real_grid": {
            "crs": rg["crs"],
            "bounds_projected": rg["bounds_projected"],
            "size_px": rg["size_px"],
            "meters_per_px": rg["meters_per_px"],
            "crop_box_px": list(cw.crop_box_px),
        },
        "height_tiles": relief.height_tiles_meta(fine),
        "sources": {
            "dem": {
                "name": "ETOPO 2022 v1 15s surface (relief réel rééchantillonné, provisoire)",
                "tiles": real.get("sources", {}).get("dem", {}).get("tiles", []),
            },
            "vectors": real.get("sources", {}).get("vectors", {}),
            "composed": "cent-ans geo compose (lot CC C1)",
        },
        "generated_at": datetime.now(UTC).replace(microsecond=0).isoformat(),
    }


# ----------------------------------------------------------------------------
# Orchestration
# ----------------------------------------------------------------------------


def compile_all(
    map_dir: Path = MAP_DIR,
    reuse_warp: bool = True,
    progress: bool = True,
    board: Path | None = None,
) -> dict:
    """Compile la carte composée dans ``map_dir`` ; renvoie le rapport."""
    t0 = time.time()
    log = (
        (lambda m: print(f"[compose] {m} ({time.time() - t0:.0f} s)", flush=True))
        if progress
        else (lambda m: None)
    )
    ensure_real_sources()
    cfg = load_config()
    report: dict = {}
    cw = make_contract(progress=progress, reuse=reuse_warp)
    log(f"déformation prête, canevas {cw.size_px}")

    frame = gen.Frame(cw, 1.0)
    log("cadre composé")
    land, mask, report["land"] = build_land(frame, cw, cfg)
    # Lot C4a : lacs fins (Léman, Constance, Fucino...) perdus par le seuil de plan d'eau.
    sea_mask = (
        mask.copy()
    )  # fleuves : leurs embouchures se calent sur la mer, pas sur un lac
    report["lakes_fine"] = lakes_fine.punch(mask, cw, cfg.get("lakes_fine"), map_dir)
    terrain.write_png8(mask, map_dir / "land_mask.png")
    vectors.write_geojson(coastline_frame(land), map_dir / "coastline.geojson")
    log(f"terre : {report['land']['polygons']} polygones")

    features, report["rivers"] = build_rivers(cw, cfg, sea_mask)
    write_rivers(features, map_dir / "rivers.geojson")
    log(f"fleuves : {len(features)}")

    height = provisional_heightmap(frame, mask)
    terrain.write_png16(terrain.height_to_uint16(height), map_dir / "heightmap.png")
    log("relief provisoire")

    meta = json.loads(real_path("map.json").read_text("utf-8"))
    raw = terrain.read_png16(real_path("heightmap.png")).astype(np.float32)
    real_m = (
        meta["height_min_m"]
        + raw * (meta["height_max_m"] - meta["height_min_m"]) / 65535.0
    )
    del raw
    ridges, report["ridges"] = build_ridges(cw, cfg, real_m)
    report["ridges"]["authored_kept"] = keep_authored_ridges(ridges)
    (COMPOSE_DIR / "ridges.json").write_text(
        json.dumps(ridges, ensure_ascii=False, separators=(",", ":")) + "\n",
        encoding="utf-8",
    )
    log(f"crêtes : {report['ridges']['ridges']}")

    (map_dir / "map.json").write_text(
        json.dumps(composed_map_json(cw), indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    report["seconds"] = round(time.time() - t0, 1)
    if board is not None:
        make_board(map_dir, board)
    return report


def write_rivers(features: list[dict], path: Path) -> None:
    """``rivers.geojson`` (pixels composés, une décimale)."""
    import geopandas as gpd

    props = [{k: v for k, v in f.items() if k != "coords"} for f in features]
    geometry = [LineString(f["coords"]) for f in features]
    frame = gpd.GeoDataFrame(props, geometry=geometry, crs=None)
    vectors.write_geojson(frame, path)


def make_board(map_dir: Path, out_dir: Path, width: int = 1600) -> Path:
    """Planche 2D de contrôle : relief provisoire, côte, fleuves, crêtes."""
    from PIL import ImageDraw

    out_dir.mkdir(parents=True, exist_ok=True)
    mask = np.asarray(Image.open(map_dir / "land_mask.png")) > 0
    h, w = mask.shape
    scale = width / w
    size = (width, int(h * scale))
    land = Image.fromarray((mask * 255).astype(np.uint8)).resize(size, Image.BOX)
    base = np.asarray(land, dtype=np.float32) / 255.0
    height = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    hs = np.asarray(
        Image.fromarray(np.clip(height / 2500.0, 0, 1).astype(np.float32)).resize(
            size, Image.BOX
        )
    )
    rgb = np.zeros((*base.shape, 3), dtype=np.float32)
    rgb[...] = np.array([0.55, 0.70, 0.85])
    ground = np.stack([0.80 - 0.35 * hs, 0.78 - 0.30 * hs, 0.60 - 0.30 * hs], axis=-1)
    rgb = rgb * (1 - base[..., None]) + ground * base[..., None]
    img = Image.fromarray((np.clip(rgb, 0, 1) * 255).astype(np.uint8))
    draw = ImageDraw.Draw(img)
    for feat in json.loads((map_dir / "rivers.geojson").read_text("utf-8"))["features"]:
        pts = [(x * scale, y * scale) for x, y in feat["geometry"]["coordinates"]]
        draw.line(pts, fill=(40, 90, 200), width=2)
    for feat in json.loads((map_dir / "coastline.geojson").read_text("utf-8"))[
        "features"
    ]:
        pts = [(x * scale, y * scale) for x, y in feat["geometry"]["coordinates"]]
        draw.line(pts, fill=(30, 30, 30), width=1)
    ridges = json.loads((COMPOSE_DIR / "ridges.json").read_text("utf-8"))["ridges"]
    for ridge in ridges:
        color = (120, 60, 30) if ridge["kind"] == "massif" else (190, 150, 90)
        draw.line(
            [(x * scale, y * scale) for x, y in ridge["points"]], fill=color, width=2
        )
        for p in ridge["passes"]:
            x, y = p["at"][0] * scale, p["at"][1] * scale
            draw.ellipse((x - 3, y - 3, x + 3, y + 3), outline=(255, 255, 255))
    path = out_dir / "planche_c1.png"
    img.save(path)
    return path
