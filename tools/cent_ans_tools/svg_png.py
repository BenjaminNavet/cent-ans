"""Rasterise the SVG-only game icons to ink PNGs for the Bevy client.

Bevy cannot load SVG. ``game/assets/icons/icons.json`` lists 250 ids; those that no PNG family
(``icons/ink``, ``icons/entity``, ``ui/medallions``) covers exist only as SVG (the 12 ``skill_*``
icons). This tool renders each of them with ``resvg`` at 128 px and writes it in the ink family:
alpha = the drawing, RGB = ``INK`` (``HudStyle.INK``), exactly what ``IconLibrary.tint`` expects, so
the PNG tints like any other ink icon. ``icons/ink/index.json`` gets the new ids.

``EXTRA_ICONS`` adds SVG files that scripts load directly by path (the campaign ruins marker,
``ruin_markers.gd``). The six painted battle cursors already exist as PNG (``ui/cursors``); their
SVG are only the Godot fallback, so nothing is converted for them.
"""

from __future__ import annotations

import io
import json
from pathlib import Path

import resvg_py
from PIL import Image

from cent_ans_tools.ink_icons import INK_RGB
from cent_ans_tools.paths import REPO_DIR

ICONS_DIR = REPO_DIR / "game" / "assets" / "icons"
INK_DIR = ICONS_DIR / "ink"
SIZE = 128
# Ids not in icons.json, loaded by path in GDScript: id -> SVG file name.
EXTRA_ICONS = {"marker_ruins": "delapouite-castle-ruins.svg"}
# PNG families that take precedence over the SVG in ``IconLibrary``.
PNG_INDEXES = (
    ICONS_DIR / "ink" / "index.json",
    ICONS_DIR / "entity" / "index.json",
    REPO_DIR / "game" / "assets" / "ui" / "medallions" / "index.json",
)


def _read_ids(path: Path) -> dict[str, str]:
    if not path.is_file():
        return {}
    return json.loads(path.read_text(encoding="utf-8")).get("icons", {})


def svg_only_icons(icons_dir: Path = ICONS_DIR) -> dict[str, str]:
    """Ids of ``icons.json`` that no PNG family covers -> SVG file name."""
    table = json.loads((icons_dir / "icons.json").read_text(encoding="utf-8"))["icons"]
    covered: set[str] = set()
    for index in (
        icons_dir / "ink" / "index.json",
        icons_dir / "entity" / "index.json",
    ):
        covered |= set(_read_ids(index))
    covered |= set(_read_ids(PNG_INDEXES[2]))
    return {i: row["file"] for i, row in sorted(table.items()) if i not in covered}


def render_ink(svg_path: Path, size: int = SIZE) -> Image.Image:
    """``size`` px RGBA: alpha from the rendered SVG, RGB = ``INK_RGB``."""
    png = resvg_py.svg_to_bytes(svg_path=str(svg_path), width=size, height=size)
    alpha = Image.open(io.BytesIO(bytes(png))).convert("RGBA").getchannel("A")
    image = Image.new("RGBA", alpha.size, (*INK_RGB, 0))
    image.putalpha(alpha)
    return image


def build(
    icons_dir: Path = ICONS_DIR, extra: dict[str, str] | None = None
) -> list[str]:
    """Renders every SVG-only icon (and ``extra``) into ``ink/`` and updates its index."""
    ink_dir = icons_dir / "ink"
    index_path = ink_dir / "index.json"
    payload = json.loads(index_path.read_text(encoding="utf-8"))
    table = payload["icons"]
    todo = svg_only_icons(icons_dir)
    todo.update(EXTRA_ICONS if extra is None else extra)
    written = []
    for icon_id, svg_name in todo.items():
        target = ink_dir / f"{icon_id}.png"
        render_ink(icons_dir / svg_name).save(target)
        table[icon_id] = target.name
        written.append(icon_id)
    payload["icons"] = dict(sorted(table.items()))
    index_path.write_text(
        json.dumps(payload, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    return written
