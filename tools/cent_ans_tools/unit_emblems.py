"""Unit emblems (lot OMR R5): local square "paintings" for units without an illustration.

The unit cards' miniatures (``entity_icons``) are cut from painted illustrations. The
eastern units of lot OMR R5 have none and no cloud spending is allowed, so their source
picture is drawn here: the unit's game-icons.net silhouette (``icons_catalog``, CC BY 3.0,
already tinted in ``game/assets/icons/``) in gold leaf with an ink outline, on the flat
diapered azure ground of the illuminated miniatures (same palette as the common frame of
``data/ui/entity_icons.json``). ``entity_icons.build`` then frames them like the others.

Output: ``tools/emblem_src/<unit id>.png`` (512 x 512, RGB). Deterministic, no network.
"""

from __future__ import annotations

import io
import json
import re
from pathlib import Path

import numpy as np
import resvg_py
from PIL import Image, ImageFilter

REPO_DIR = Path(__file__).resolve().parents[2]
ICONS_DIR = REPO_DIR / "game" / "assets" / "icons"
OUT_DIR = REPO_DIR / "tools" / "emblem_src"
SIZE = 512

# Units drawn as emblems (ids of ``icons_catalog.ICONS``).
UNITS: tuple[str, ...] = (
    "unit_mamluk_cavalry",
    "unit_steppe_horse_archers",
    "unit_akinci",
    "unit_yaya",
    "unit_serbian_heavy_cavalry",
    "unit_pronoiars",
    "unit_druzhina",
    "unit_lithuanian_light_cavalry",
    "unit_teutonic_knights",
    "unit_almogavars",
)

AZURE = (39, 73, 140)
AZURE_DARK = (22, 42, 88)
GOLD_LIGHT = (240, 217, 138)
GOLD_DARK = (154, 111, 37)
INK = (56, 36, 18)


def icon_svg(unit_id: str) -> str:
    """The tinted game-icons SVG of ``unit_id`` (``game/assets/icons/icons.json``)."""
    table = json.loads((ICONS_DIR / "icons.json").read_text(encoding="utf-8"))
    entry = table["icons"][unit_id]
    return (ICONS_DIR / entry["file"]).read_text(encoding="utf-8")


def silhouette(svg: str, size: int) -> np.ndarray:
    """Coverage (0-1) of the icon's drawing rendered at ``size`` pixels."""
    svg = re.sub(r'\s(width|height)="[^"]*"', "", svg, count=2)
    png = resvg_py.svg_to_bytes(svg_string=svg, width=size, height=size)
    image = Image.open(io.BytesIO(bytes(png))).convert("RGBA")
    return np.asarray(image, dtype=np.float32)[:, :, 3] / 255.0


def ground(size: int) -> np.ndarray:
    """Flat azure ground with a small gold diaper (lozenges and dots), darker at the edge."""
    ys, xs = np.mgrid[0:size, 0:size].astype(np.float32)
    rgb = np.empty((size, size, 3), dtype=np.float32)
    rgb[:] = AZURE
    cell = size / 8.0
    u = (xs + ys) / cell
    v = (xs - ys) / cell
    lines = np.minimum(np.abs(u - np.round(u)), np.abs(v - np.round(v)))
    lattice = np.clip(1.0 - lines * cell / 1.6, 0.0, 1.0) * 0.35
    cu = (np.floor(u) + 0.5) * cell
    cv = (np.floor(v) + 0.5) * cell
    cx, cy = (cu + cv) / 2.0, (cu - cv) / 2.0
    dots = np.clip(1.0 - (np.hypot(xs - cx, ys - cy) - 2.5) / 1.5, 0.0, 1.0) * 0.6
    gold = np.clip(lattice + dots, 0.0, 1.0)[..., None]
    rgb = rgb * (1.0 - gold) + np.array(GOLD_DARK, dtype=np.float32) * gold
    edge = np.hypot(xs - size / 2, ys - size / 2) / (size * 0.72)
    shade = np.clip(edge, 0.0, 1.0)[..., None] ** 2
    return rgb * (1.0 - shade) + np.array(AZURE_DARK, dtype=np.float32) * shade


def emblem(unit_id: str, size: int = SIZE) -> Image.Image:
    """The emblem picture of ``unit_id``: gold silhouette, ink outline, azure ground."""
    inner = round(size * 0.74)
    mask = np.zeros((size, size), dtype=np.float32)
    offset = (size - inner) // 2
    mask[offset : offset + inner, offset : offset + inner] = silhouette(
        icon_svg(unit_id), inner
    )
    mask_image = Image.fromarray((mask * 255).astype(np.uint8))
    outline = (
        np.asarray(mask_image.filter(ImageFilter.MaxFilter(7)), dtype=np.float32)
        / 255.0
    )
    shadow = (
        np.asarray(
            mask_image.filter(ImageFilter.MaxFilter(5)).filter(
                ImageFilter.GaussianBlur(6)
            ),
            dtype=np.float32,
        )
        / 255.0
    )
    shadow = np.roll(shadow, (6, 6), axis=(0, 1))
    rgb = ground(size)
    rgb *= (1.0 - 0.45 * shadow)[..., None]
    rgb = (
        rgb * (1.0 - outline[..., None])
        + np.array(INK, np.float32) * outline[..., None]
    )
    ys = np.linspace(0.0, 1.0, size, dtype=np.float32)[:, None, None]
    gold = (
        np.array(GOLD_LIGHT, np.float32) * (1.0 - ys)
        + np.array(GOLD_DARK, np.float32) * ys
    )
    rgb = rgb * (1.0 - mask[..., None]) + gold * mask[..., None]
    return Image.fromarray(np.clip(rgb, 0, 255).astype(np.uint8), "RGB")


def build(units: tuple[str, ...] = UNITS) -> list[Path]:
    """Writes ``tools/emblem_src/<id>.png`` for ``units``; returns the paths written."""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    written = []
    for unit_id in units:
        path = OUT_DIR / f"{unit_id}.png"
        emblem(unit_id).save(path, optimize=True)
        written.append(path)
    return written
