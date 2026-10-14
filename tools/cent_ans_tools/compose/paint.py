"""Peinture vectorielle régionale de la carte composée (lot CP, ADR 0361).

``data/map/compose/paint/<region>.json`` (schéma ``compose_paint.schema.json``) liste des traits
polygonaux ``add`` / ``clear`` par couche (``forest``, ``rock``, ``fields``, ``marsh``). ``cent-ans
compose paint`` les rastérise sur la grille des masques d'auteur (1 case = 16 px composés, centre de la
case ``i`` à ``(i + 0.5) * 16`` px), bord adouci par ``feather_km`` (rampe de part et d'autre du contour),
combine les traits au ``max`` par couche et mode, puis écrit ``masks/derived/<couche>.png`` (ajout) et
``masks/derived/<couche>_clear.png`` (retrait). ``masks.load_all`` les combine (``max``) avec le pinceau
de l'éditeur, qui écrit ``masks/*.png`` sans jamais être écrasé.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

from cent_ans_tools.compose.masks import (
    CLEAR_SUFFIX,
    COMPOSE_DIR,
    MASK_CELL_PX,
    MASK_NAMES,
)
from cent_ans_tools.paths import DATA_DIR

PAINT_SCHEMA = DATA_DIR / "schemas" / "compose_paint.schema.json"
SUPERSAMPLE = 4  # sous-échantillonnage par case pour le contour et la distance
# Couleurs de l'aperçu (RGB) par couche.
PREVIEW_COLORS = {
    "forest": (20, 130, 40),
    "rock": (150, 110, 80),
    "fields": (230, 200, 40),
    "marsh": (40, 120, 220),
}


class PaintError(ValueError):
    """Fichier de peinture invalide."""


def validate(doc: dict, source: str = "") -> None:
    """Valide ``doc`` contre le schéma ; lève :class:`PaintError` avec les messages."""
    import jsonschema

    schema = json.loads(PAINT_SCHEMA.read_text(encoding="utf-8"))
    errors = sorted(
        jsonschema.Draft202012Validator(schema).iter_errors(doc),
        key=lambda e: list(e.path),
    )
    if errors:
        raise PaintError(
            "; ".join(
                f"{source}{'/'.join(map(str, e.path))}: {e.message}" for e in errors
            )
        )


def load_strokes(paint_dir: Path = COMPOSE_DIR / "paint") -> list[dict]:
    """Traits de tous les ``paint/*.json`` (validés), avec la clé ajoutée ``region`` (nom du fichier)."""
    strokes: list[dict] = []
    seen: set[str] = set()
    for path in sorted(paint_dir.glob("*.json")):
        doc = json.loads(path.read_text(encoding="utf-8"))
        validate(doc, f"{path.name}:")
        for stroke in doc["strokes"]:
            key = f"{path.stem}/{stroke['id']}"
            if key in seen:
                raise PaintError(f"{path.name}: id de trait en double : {stroke['id']}")
            seen.add(key)
            strokes.append({**stroke, "region": path.stem})
    return strokes


def stroke_polygon_px(stroke: dict, warp=None) -> np.ndarray:
    """Sommets du trait en pixels composés ``(n, 2)`` ; ``lonlat`` passe par le cartogramme ``warp``."""
    pts = np.asarray(stroke["polygon"], dtype=np.float64)
    if stroke.get("coords", "lonlat") == "px":
        return pts
    if warp is None:
        from cent_ans_tools.compose.contract import ComposedWarp

        warp = ComposedWarp.load()
    xs, ys = warp.forward(pts[:, 0], pts[:, 1])
    return np.stack([np.asarray(xs), np.asarray(ys)], axis=1)


def rasterize_polygon(
    polygon_px: np.ndarray,
    shape: tuple[int, int],
    feather_px: float = 0.0,
) -> np.ndarray:
    """Polygone (px composés) -> grille de cases ``shape`` = (rows, cols), valeurs [0, 1].

    Dedans = 1 ; ``feather_px`` (px composés) : rampe lisse (smoothstep) de ``-f`` à ``+f`` de part et
    d'autre du contour, donc 0,5 sur le contour.
    """
    rows, cols = shape
    out = np.zeros(shape, dtype=np.float32)
    # Coordonnées en unités de case, décalées pour que le centre de la case i soit à i + 0.5.
    cell = polygon_px / MASK_CELL_PX
    pad = feather_px / MASK_CELL_PX + 1.0
    x0 = max(0, int(np.floor(cell[:, 0].min() - pad)))
    x1 = min(cols, int(np.ceil(cell[:, 0].max() + pad)))
    y0 = max(0, int(np.floor(cell[:, 1].min() - pad)))
    y1 = min(rows, int(np.ceil(cell[:, 1].max() + pad)))
    if x1 <= x0 or y1 <= y0:
        return out
    s = SUPERSAMPLE
    image = Image.new("L", ((x1 - x0) * s, (y1 - y0) * s), 0)
    sub = [((x - x0) * s, (y - y0) * s) for x, y in cell]
    ImageDraw.Draw(image).polygon(sub, fill=255)
    inside = np.asarray(image) > 127
    if feather_px > 0:
        px_per_sub = MASK_CELL_PX / s
        dist = (
            ndimage.distance_transform_edt(inside)
            - ndimage.distance_transform_edt(~inside)
        ) * px_per_sub
        t = np.clip((dist + feather_px) / (2.0 * feather_px), 0.0, 1.0)
        values = t * t * (3.0 - 2.0 * t)
    else:
        values = inside.astype(np.float32)
    h, w = y1 - y0, x1 - x0
    block = values.reshape(h, s, w, s).mean(axis=(1, 3))
    out[y0:y1, x0:x1] = block
    return out


def rasterize_all(
    strokes: list[dict],
    shape: tuple[int, int],
    warp=None,
    meters_per_px: float | None = None,
) -> dict[str, np.ndarray]:
    """Grilles ``{couche}`` (ajout) et ``{couche}_clear`` (retrait), max des traits, seulement les non vides."""
    if meters_per_px is None:
        meters_per_px = _composed_mpp()
    found: dict[str, np.ndarray] = {}
    for stroke in strokes:
        key = stroke["layer"] + (CLEAR_SUFFIX if stroke["mode"] == "clear" else "")
        feather_px = float(stroke.get("feather_km", 0.0)) * 1000.0 / meters_per_px
        grid = rasterize_polygon(stroke_polygon_px(stroke, warp), shape, feather_px)
        grid *= float(stroke.get("strength", 1.0))
        found[key] = grid if key not in found else np.maximum(found[key], grid)
    return {key: grid for key, grid in found.items() if grid.any()}


def _composed_mpp() -> float:
    meta = json.loads((DATA_DIR / "map" / "map.json").read_text(encoding="utf-8"))
    return float(meta["meters_per_px"])


def mask_shape(compose_dir: Path = COMPOSE_DIR) -> tuple[int, int]:
    """``(rows, cols)`` des masques (cases de 16 px sur ``size_px`` de ``ridges.json``)."""
    width, height = json.loads((compose_dir / "ridges.json").read_text("utf-8"))[
        "size_px"
    ]
    return -(-int(height) // MASK_CELL_PX), -(-int(width) // MASK_CELL_PX)


def write_derived(compose_dir: Path = COMPOSE_DIR, warp=None) -> dict[str, int]:
    """Rastérise ``paint/*.json`` vers ``masks/derived/`` ; renvoie le nombre de traits par fichier écrit.

    Les fichiers dérivés que plus aucun trait n'alimente sont supprimés (le pinceau n'est pas touché).
    """
    strokes = load_strokes(compose_dir / "paint")
    grids = rasterize_all(strokes, mask_shape(compose_dir), warp)
    out_dir = compose_dir / "masks" / "derived"
    counts: dict[str, int] = {}
    for stroke in strokes:
        key = stroke["layer"] + (CLEAR_SUFFIX if stroke["mode"] == "clear" else "")
        counts[key] = counts.get(key, 0) + 1
    out_dir.mkdir(parents=True, exist_ok=True)
    for name in MASK_NAMES:
        for key in (name, name + CLEAR_SUFFIX):
            path = out_dir / f"{key}.png"
            if key in grids:
                data = np.rint(grids[key] * 255.0).astype(np.uint8)
                Image.fromarray(data, mode="L").save(path, optimize=True)
            elif path.exists():
                path.unlink()
    return counts


def preview(
    out_path: Path,
    compose_dir: Path = COMPOSE_DIR,
    land_mask_path: Path | None = None,
    warp=None,
) -> Path:
    """Image couleur de la grille des masques : traits par couche sur la terre (gris) et la mer (sombre).

    ``add`` = couleur de la couche, ``clear`` = damier couche / noir. Pour contrôle sans lancer le jeu.
    """
    shape = mask_shape(compose_dir)
    rows, cols = shape
    land_path = land_mask_path or (DATA_DIR / "map" / "land_mask.png")
    image = np.full((rows, cols, 3), 30.0, dtype=np.float32)
    if land_path.exists():
        Image.MAX_IMAGE_PIXELS = None
        with Image.open(land_path) as handle:
            land = np.asarray(
                handle.convert("L").resize((cols, rows), Image.Resampling.BOX),
                dtype=np.float32,
            )
        image = np.where((land / 255.0)[..., None] > 0.5, 150.0, 30.0) * np.ones(3)
    grids = rasterize_all(load_strokes(compose_dir / "paint"), shape, warp)
    checker = ((np.arange(rows)[:, None] + np.arange(cols)[None, :]) % 2 == 0)[
        ..., None
    ]
    for name in MASK_NAMES:
        color = np.asarray(PREVIEW_COLORS[name], dtype=np.float32)
        add = grids.get(name)
        if add is not None:
            image = image * (1 - add[..., None]) + color * add[..., None]
        clear = grids.get(name + CLEAR_SUFFIX)
        if clear is not None:
            tint = np.where(checker, color, 0.0)
            image = image * (1 - clear[..., None]) + tint * clear[..., None]
    out_path.parent.mkdir(parents=True, exist_ok=True)
    Image.fromarray(np.clip(image, 0, 255).astype(np.uint8), mode="RGB").save(out_path)
    return out_path
