"""Draws the plain ink glyph icons of the HUD (audit XP-LISIBLE, M6) and registers them.

Hand-made line drawings (no model, no cost): dark ink on parchment at 512 px, then the same
``process_icon`` as the generated icons (crop, stroke weight normalised) so that they sit in the
family. Output: ``game/assets/icons/ink/<id>.png``, ``tools/da5_raw/icons/<id>.jpg`` (source) and
the entries of ``data/ui/icons_ink.json`` and ``game/assets/icons/ink/index.json``.

Run: ``uv run --project tools python tools/make_ink_glyphs.py``.
"""

import json
from pathlib import Path

from PIL import Image, ImageDraw

from cent_ans_tools import ink_icons

REPO = Path(__file__).resolve().parents[1]
CANVAS = 512
PARCHMENT = (232, 214, 170)
INK = (56, 36, 18)
STROKE = 30


def _canvas() -> tuple[Image.Image, ImageDraw.ImageDraw]:
    image = Image.new("RGB", (CANVAS, CANVAS), PARCHMENT)
    return image, ImageDraw.Draw(image)


def _line(draw: ImageDraw.ImageDraw, points: list[tuple[int, int]], width: int = STROKE) -> None:
    """Polyline with round joins and caps."""
    draw.line(points, fill=INK, width=width, joint="curve")
    radius = width // 2
    for x, y in (points[0], points[-1]):
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=INK)


def arrow(direction: str) -> Image.Image:
    """Bold arrow: a shaft and a chevron head."""
    image, draw = _canvas()
    low, high, mid = 90, 422, 256
    if direction in ("up", "down"):
        tip, tail = (high, low) if direction == "down" else (low, high)
        step = 110 if direction == "down" else -110
        _line(draw, [(mid, tail), (mid, tip)])
        _line(draw, [(mid - 120, tip - step), (mid, tip), (mid + 120, tip - step)])
    else:
        tip, tail = (high, low) if direction == "right" else (low, high)
        step = 110 if direction == "right" else -110
        _line(draw, [(tail, mid), (tip, mid)])
        _line(draw, [(tip - step, mid - 120), (tip, mid), (tip - step, mid + 120)])
    return image


def check() -> Image.Image:
    image, draw = _canvas()
    _line(draw, [(90, 270), (200, 390), (430, 120)], width=STROKE + 6)
    return image


def cross() -> Image.Image:
    image, draw = _canvas()
    # Thinner than the other glyphs: the crossing must stay narrower than the paper-estimate
    # window of ``ink_alpha`` or it would read as paper.
    _line(draw, [(110, 110), (402, 402)], width=24)
    _line(draw, [(402, 110), (110, 402)], width=24)
    return image


def book() -> Image.Image:
    """Open book: two pages meeting at the spine, a few lines of text on each."""
    image, draw = _canvas()
    _line(draw, [(256, 150), (256, 400)], width=24)
    _line(draw, [(256, 150), (120, 120), (60, 140), (60, 380), (120, 360), (256, 400)], width=24)
    _line(draw, [(256, 150), (392, 120), (452, 140), (452, 380), (392, 360), (256, 400)], width=24)
    for offset in (190, 245, 300):
        _line(draw, [(105, offset), (215, offset + 12)], width=16)
        _line(draw, [(297, offset + 12), (407, offset)], width=16)
    return image


ICONS = {
    "livre": ("Livre", "an open book with lines of text", book, ["livre"]),
    "hausse": ("Hausse", "a bold arrow pointing up", lambda: arrow("up"), ["hausse"]),
    "baisse": ("Baisse", "a bold arrow pointing down", lambda: arrow("down"), ["baisse"]),
    "oui": ("Oui", "a bold check mark", check, ["oui"]),
    "non": ("Non", "a bold saltire cross", cross, ["non"]),
    "fleche_droite": (
        "Flèche à droite",
        "a bold arrow pointing right",
        lambda: arrow("right"),
        ["fleche_droite"],
    ),
    "fleche_gauche": (
        "Flèche à gauche",
        "a bold arrow pointing left",
        lambda: arrow("left"),
        ["fleche_gauche"],
    ),
}
# Existing generated glyphs also reachable under the audit's French names.
ALIASES = {"epees": "glyph_swords", "etoile": "glyph_star", "croix": "glyph_cross"}


def main() -> None:
    catalog_path = REPO / "data" / "ui" / "icons_ink.json"
    catalog_text = catalog_path.read_text(encoding="utf-8")
    catalog = json.loads(catalog_text)
    index_path = ink_icons.ICONS_OUT_DIR / "index.json"
    index = json.loads(index_path.read_text(encoding="utf-8"))
    known = {item["id"] for item in catalog["icons"]}
    raw_dir = REPO / "tools" / "da5_raw" / "icons"
    new_lines = []
    for icon_id, (label, subject, draw_icon, targets) in ICONS.items():
        raw = draw_icon()
        raw.save(raw_dir / f"{icon_id}.jpg", quality=95)
        image, width = ink_icons.process_icon(raw, catalog["icon_style"]["size"])
        image.save(ink_icons.ICONS_OUT_DIR / f"{icon_id}.png", optimize=True)
        index["stroke_px"][icon_id] = round(width, 2)
        for target in targets:
            index["icons"][target] = f"{icon_id}.png"
        if icon_id not in known:
            entry = {
                "id": icon_id,
                "label": label,
                "group": "glyph",
                "subject": subject,
                "targets": targets,
            }
            new_lines.append("    " + json.dumps(entry, ensure_ascii=False))
    if new_lines:
        # Text insertion keeps the one-entry-per-line layout of the catalogue.
        anchor = catalog["icons"][-1]["id"]
        lines = catalog_text.split("\n")
        at = next(i for i, line in enumerate(lines) if f'"id": "{anchor}"' in line)
        lines[at] += ","
        lines[at + 1 : at + 1] = [line + "," for line in new_lines[:-1]] + [new_lines[-1]]
        catalog_path.write_text("\n".join(lines), encoding="utf-8")
    for alias, source_id in ALIASES.items():
        index["icons"][alias] = index["icons"][source_id]
    index["icons"] = dict(sorted(index["icons"].items()))
    index_path.write_text(
        json.dumps(index, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
    )
    print("wrote", ", ".join(ICONS), "and aliases", ", ".join(ALIASES))


if __name__ == "__main__":
    main()
