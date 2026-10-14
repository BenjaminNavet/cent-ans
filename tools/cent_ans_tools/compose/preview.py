"""Planche 2D de la carte composée (lot CC, C0b) : parchemin sobre.

Mer gris-bleu, terre beige, crêtes brunes épaisses, fleuves bleus, frontières de
province fines, lieux en points par type, noms des rangs 3-4 de
``settlement_markers.json``. Sorties hors dépôt (``~/dev/cent-ans-raw/compose``).
"""

from __future__ import annotations

import json
import time
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFont
from scipy import ndimage

from cent_ans_tools.compose import generalise as gen
from cent_ans_tools.compose.warp import (
    COMPOSE_DIR,
    MAP_DIR,
    Warp,
    build_warp,
    load_places,
    load_settlements,
    uniformity,
    zone_factors,
)

OUT_DIR = Path.home() / "dev" / "cent-ans-raw" / "compose"

MARGIN = (214, 203, 180)
SEA = (160, 178, 190)
LAND = (228, 212, 174)
COAST = (120, 100, 75)
RIVER = (74, 118, 168)
RIDGE = (104, 68, 36)
HILL = (176, 144, 100)
BORDER = (150, 122, 96)
DOT = (110, 30, 25)
TEXT = (50, 30, 20)


def warp_cache_path(strength: float, sigma_km: float, sea_density: float) -> Path:
    """Chemin du ``.npz`` d'une déformation (ignoré par git, voir ``compose/.gitignore``)."""
    return COMPOSE_DIR / f"warp_s{strength:.2f}_g{sigma_km:.0f}_m{sea_density:.2f}.npz"


def get_warp(
    strength: float, sigma_km: float, sea_density: float, places: list[dict]
) -> Warp:
    """Déformation relue du cache ou calculée (et écrite)."""
    path = warp_cache_path(strength, sigma_km, sea_density)
    if path.exists():
        return Warp.load(path)
    warp = build_warp(strength, sigma_km, sea_density, places=places, progress=True)
    warp.save(path)
    return warp


def _rank_names() -> dict[str, int]:
    data = json.loads((MAP_DIR / "settlement_markers.json").read_text("utf-8"))
    out = {}
    for rule in data["rank_rules"]:
        for sid in rule.get("ids", []):
            out[sid] = rule["rank"]
    return out


def _font(size: int):
    for name in ("Georgia.ttf", "Times New Roman.ttf", "Palatino.ttc"):
        for base in ("/System/Library/Fonts/Supplemental/", "/Library/Fonts/"):
            try:
                return ImageFont.truetype(base + name, size)
            except OSError:
                continue
    return ImageFont.load_default(size)


def render_board(
    warp: Warp,
    places: list[dict],
    out_path: Path,
    title: str,
    width: int = 1600,
    scale: float = 0.5,
) -> dict:
    """Dessine la planche de ``warp`` ; renvoie les temps et comptes."""
    t0 = time.time()
    frame = gen.Frame(warp, scale)
    t_frame = time.time()
    land, land_stats = gen.generalise_land(frame)
    rivers = gen.generalise_rivers(warp)
    massif = gen.generalise_ridges(frame, "massif")
    hills = gen.generalise_ridges(frame, "hills", min_length_km=90.0)
    labels = gen.province_frame_labels(frame)
    border = ndimage.binary_dilation(gen.border_mask(labels), iterations=1)
    t_gen = time.time()

    ys, xs = np.nonzero(frame.inside)
    bx0, bx1, by0, by1 = xs.min(), xs.max() + 1, ys.min(), ys.max() + 1
    w, h = int(bx1 - bx0), int(by1 - by0)

    def to_img(xy):  # pixels composés -> pixels de la planche
        a = np.asarray(xy, dtype=float)
        return [(float(x) * scale - bx0, float(y) * scale - by0) for x, y in a]

    img = Image.new("RGB", (w, h), MARGIN)
    inside = frame.inside[by0:by1, bx0:bx1]
    arr = np.asarray(img).copy()
    arr[inside] = SEA
    img = Image.fromarray(arr)
    draw = ImageDraw.Draw(img)
    for poly in land.geoms:
        draw.polygon(to_img(poly.exterior.coords), fill=LAND)
        for ring in poly.interiors:
            draw.polygon(to_img(ring.coords), fill=SEA)
    arr = np.asarray(img).copy()
    on_land = np.zeros((h, w), dtype=bool)
    mask_img = Image.new("L", (w, h), 0)
    mdraw = ImageDraw.Draw(mask_img)
    for poly in land.geoms:
        mdraw.polygon(to_img(poly.exterior.coords), fill=255)
    on_land = np.asarray(mask_img) > 0
    b = border[by0:by1, bx0:bx1] & on_land
    arr[b] = BORDER
    img = Image.fromarray(arr)
    draw = ImageDraw.Draw(img)
    for poly in land.geoms:
        draw.line(to_img(poly.exterior.coords), fill=COAST, width=2, joint="curve")
    for _, line in rivers:
        draw.line(to_img(line.coords), fill=RIVER, width=2, joint="curve")
    for line in hills:
        draw.line(to_img(line.coords), fill=HILL, width=4, joint="curve")
    for line in massif:
        draw.line(to_img(line.coords), fill=RIDGE, width=8, joint="curve")

    # lieux
    ranks = _rank_names()
    names = load_settlements()
    font = _font(max(14, w // 110))
    lon = np.array([p["lon"] for p in places])
    lat = np.array([p["lat"] for p in places])
    cx, cy = warp.forward(lon, lat)
    for p, x, y in zip(places, cx, cy, strict=True):
        ix, iy = float(x) * scale - bx0, float(y) * scale - by0
        if not (0 <= ix < w and 0 <= iy < h) or not inside[int(iy), int(ix)]:
            continue
        kind = p.get("kind", "city")
        r = {"city": 6, "town": 4.5, "castle": 4, "abbey": 4, "village": 2.5}.get(
            kind, 3
        )
        if kind == "castle":
            draw.rectangle([ix - r, iy - r, ix + r, iy + r], fill=DOT)
        elif kind == "abbey":
            draw.polygon(
                [(ix, iy - r - 1), (ix - r, iy + r), (ix + r, iy + r)], fill=DOT
            )
        else:
            draw.ellipse(
                [ix - r, iy - r, ix + r, iy + r], fill=DOT, outline=(240, 230, 200)
            )
        if ranks.get(p["id"], 0) >= 3 and p["id"] in names:
            label = names[p["id"]]["name"]["display"]
            draw.text(
                (ix + r + 3, iy - 9),
                label,
                fill=TEXT,
                font=font,
                stroke_width=2,
                stroke_fill=LAND,
            )
    draw.text((20, 14), title, fill=TEXT, font=_font(max(18, w // 80)))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    final = img.resize((width, int(round(h * width / w))), Image.LANCZOS)
    final.save(out_path, optimize=True)
    return {
        "path": str(out_path),
        "size": final.size,
        "frame_s": round(t_frame - t0, 1),
        "generalise_s": round(t_gen - t_frame, 1),
        "total_s": round(time.time() - t0, 1),
        "islands": land_stats["islands"],
        "rivers": len(rivers),
        "massif_lines": len(massif),
        "hills_lines": len(hills),
    }


def make_boards(
    strengths: tuple[float, ...] = (0.5, 1.0),
    sigma_km: float = 150.0,
    sea_density: float = 0.6,
    real: bool = True,
    width: int = 1600,
    out_dir: Path = OUT_DIR,
) -> dict:
    """Produit une planche par ``strength`` plus la géographie réelle recadrée."""
    places, origin = load_places()
    report = {"places_origin": origin, "places": len(places)}
    for s in strengths:
        warp = get_warp(s, sigma_km, sea_density, places)
        info = render_board(
            warp,
            places,
            out_dir / f"planche_s{s:.2f}.png",
            f"Carte composée — strength {s:.2f}, sigma {sigma_km:.0f} km, mer {sea_density:.2f}",
            width,
        )
        info["warp"] = warp.params
        info["uniformity"] = uniformity(warp, places)
        info["zones"] = zone_factors(warp)
        report[f"s{s:.2f}"] = info
    if real:
        warp = Warp.identity()
        report["real"] = render_board(
            warp,
            places,
            out_dir / "planche_reelle.png",
            "Géographie réelle recadrée",
            width,
        )
    (out_dir / "rapport.json").write_text(
        json.dumps(report, ensure_ascii=False, indent=1, default=str), "utf-8"
    )
    return report
