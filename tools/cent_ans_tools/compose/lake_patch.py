"""Remet à leur vrai niveau les lacs que l'aplanissement du relief composé a laissés flottants (lot mapgeo).

``flatten_lakes`` (``compose/relief.py``) cale un lac au 10e centile de son anneau de rive. Quand le
relief réel (Copernicus, déformé) garde une partie du lac à son niveau véritable, plus bas, le lac
du masque est plat à un niveau bien trop haut : la nappe flotte au-dessus des versants (Léman à
607 m pour 372 m réels, lac de Garde à 405 m pour 65 m) et laisse un trou noir là où le fond réel
reste visible.

Ce module lit ``heightmap_render.png`` : si l'anneau de rive d'un lac porte un palier plat
nettement plus bas que ``level_m`` (au moins ``MIN_PLATEAU_PX`` pixels à la même hauteur au demi-mètre),
ce palier est le vrai niveau. Les pixels du relief (E0, pyramide E1-E3, ``heightmap*.png``) qui valent
l'ancien niveau dans la boîte du lac prennent le nouveau, et ``lakes.json`` reçoit ``level_m`` et
``level_patched_from`` (rejouable : un lac déjà corrigé n'est plus touché ; une chaîne de lacs voisins se cale en quelques passes, jusqu'à un rapport vide).
"""

from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

from cent_ans_tools.geo import terrain

Image.MAX_IMAGE_PIXELS = None

TILE_PX = 512


@dataclass(frozen=True)
class LakeRules:
    """Seuils de détection d'un lac flottant (hauteurs en mètres, tailles en pixels carte)."""

    min_gap_m: float = 8.0
    min_plateau_px: int = 30
    min_new_level_m: float = 3.0
    ring_px: int = 6
    box_margin_px: int = 8
    tolerance_m: float = 0.1


def lake_window(poly: list[list[float]], margin: int) -> tuple[int, int, int, int]:
    """Boîte ``(x0, y0, x1, y1)`` entière autour du contour, élargie de ``margin`` pixels."""
    xs = [p[0] for p in poly]
    ys = [p[1] for p in poly]
    return (
        max(int(min(xs)) - margin, 0),
        max(int(min(ys)) - margin, 0),
        int(max(xs)) + margin + 1,
        int(max(ys)) + margin + 1,
    )


def detect_plateau(heights: np.ndarray, lake: dict, rules: LakeRules) -> float | None:
    """Niveau réel d'un lac flottant (palier plat sous sa nappe, dans l'anneau de rive), sinon ``None``."""
    poly = lake["polygon_px"]
    x0, y0, x1, y1 = lake_window(poly, rules.box_margin_px)
    sub = heights[y0:y1, x0:x1]
    mask_img = Image.new("L", (sub.shape[1], sub.shape[0]), 0)
    ImageDraw.Draw(mask_img).polygon([(p[0] - x0, p[1] - y0) for p in poly], fill=1)
    inside = np.asarray(mask_img).astype(bool)
    ring = ndimage.binary_dilation(inside, iterations=rules.ring_px) & ~inside
    values = np.round(sub[ring] / 0.5) * 0.5
    lower = values[values < lake["level_m"] - rules.min_gap_m]
    if lower.size == 0:
        return None
    levels, counts = np.unique(lower, return_counts=True)
    best = int(counts.argmax())
    if counts[best] < rules.min_plateau_px or levels[best] < rules.min_new_level_m:
        return None
    return float(levels[best])


def plan(map_dir: Path, rules: LakeRules | None = None) -> list[dict]:
    """Lacs à corriger : ``{index, id, name, old, new, box}`` (boîte en pixels carte)."""
    rules = rules or LakeRules()
    doc = json.loads((map_dir / "lakes.json").read_text(encoding="utf-8"))
    heights = terrain.uint16_to_height(
        np.asarray(Image.open(map_dir / "heightmap_render.png"))
    )
    out = []
    for index, lake in enumerate(doc["lakes"]):
        if "level_patched_from" in lake:
            continue
        new = detect_plateau(heights, lake, rules)
        if new is None:
            continue
        out.append(
            {
                "index": index,
                "id": lake["id"],
                "name": lake["name"],
                "old": lake["level_m"],
                "new": new,
                "box": lake_window(lake["polygon_px"], rules.box_margin_px),
            }
        )
    return out


def retouch(
    metres: np.ndarray, fx0: int, fy0: int, scale: int, items: list[dict], tol: float
) -> np.ndarray:
    """Remplace dans une fenêtre fine (coin ``(fx0, fy0)``) l'ancien niveau de chaque lac par le nouveau."""
    out = metres.copy()
    h, w = metres.shape
    for item in items:
        bx0, by0, bx1, by1 = (v * scale for v in item["box"])
        lx0, ly0 = max(bx0 - fx0, 0), max(by0 - fy0, 0)
        lx1, ly1 = min(bx1 - fx0, w), min(by1 - fy0, h)
        if lx1 <= lx0 or ly1 <= ly0:
            continue
        view = out[ly0:ly1, lx0:lx1]
        view[np.abs(view - item["old"]) <= tol] = item["new"]
    return out


def _patch_png(
    path: Path, items: list[dict], scale: int, tol: float, dry: bool
) -> bool:
    if not path.exists():
        return False
    tile = np.asarray(Image.open(path)).astype(np.uint16)
    return _patch_array(tile, path, 0, 0, items, scale, tol, dry)


def _patch_array(
    tile: np.ndarray,
    path: Path,
    fx0: int,
    fy0: int,
    items: list[dict],
    scale: int,
    tol: float,
    dry: bool,
) -> bool:
    metres = terrain.uint16_to_height(tile)
    encoded = terrain.height_to_uint16(retouch(metres, fx0, fy0, scale, items, tol))
    if np.array_equal(encoded, tile):
        return False
    if not dry:
        terrain.write_png16(encoded, path)
    return True


def _patch_tiles(
    tiles_dir: Path, pattern: str, scale: int, items: list[dict], tol: float, dry: bool
) -> int:
    done = 0
    cells: set[tuple[int, int]] = set()
    for item in items:
        x0, y0, x1, y1 = (v * scale for v in item["box"])
        for row in range(y0 // TILE_PX, (y1 - 1) // TILE_PX + 1):
            for col in range(x0 // TILE_PX, (x1 - 1) // TILE_PX + 1):
                cells.add((col, row))
    for col, row in sorted(cells):
        path = tiles_dir / pattern.format(col=col, row=row)
        if not path.exists():
            continue
        tile = np.asarray(Image.open(path)).astype(np.uint16)
        done += _patch_array(
            tile, path, col * TILE_PX, row * TILE_PX, items, scale, tol, dry
        )
    return done


def apply(
    map_dir: Path,
    pyramid_dir: Path | None,
    rules: LakeRules | None = None,
    dry_run: bool = False,
) -> dict:
    """Corrige relief et ``lakes.json`` ; renvoie un rapport (rejouable)."""
    rules = rules or LakeRules()
    items = plan(map_dir, rules)
    report: dict = {
        "lakes": [{k: v for k, v in i.items() if k != "box"} for i in items]
    }
    if not items:
        return report
    meta = json.loads((map_dir / "map.json").read_text(encoding="utf-8"))[
        "height_tiles"
    ]
    tiles = {
        "E0": _patch_tiles(
            map_dir / meta["dir"], meta["pattern"], 2, items, rules.tolerance_m, dry_run
        )
    }
    if pyramid_dir is not None:
        for level, scale in ((1, 4), (2, 8), (3, 16)):
            tiles[f"E{level}"] = _patch_tiles(
                pyramid_dir / f"E{level}",
                "{col}_{row}.png",
                scale,
                items,
                rules.tolerance_m,
                dry_run,
            )
    report["tiles"] = tiles
    report["images"] = {
        name: _patch_png(map_dir / name, items, 1, rules.tolerance_m, dry_run)
        for name in ("heightmap.png", "heightmap_render.png")
    }
    if not dry_run:
        path = map_dir / "lakes.json"
        doc = json.loads(path.read_text(encoding="utf-8"))
        for item in items:
            lake = doc["lakes"][item["index"]]
            lake["level_patched_from"] = lake["level_m"]
            lake["level_m"] = round(item["new"], 1)
        path.write_text(
            json.dumps(doc, ensure_ascii=False, separators=(",", ":")) + "\n",
            encoding="utf-8",
        )
    return report
