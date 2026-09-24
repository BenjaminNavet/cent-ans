"""Procedural heraldry: one 128x128 shield PNG per faction, drawn with Pillow.

The blazon (``heraldry.blazon`` in ``data/factions/*.json``) is interpreted with a
small keyword grammar: the field tincture comes from the first words (``D'or``,
``De gueules``...), falling back to ``primary_color``; charges (``semé de fleurs de
lis``, ``écartelé``, ``lion``, ``léopards``, ``aigle``, ``pals``, ``bandé``, ``croix``,
``chaînes``, ``clefs``, ``écussons``, ``guivre``, ``hermine``, ``bordure``,
``trescheur``) are drawn in ``secondary_color`` unless the blazon names another
tincture. Everything is drawn at 4x resolution then downsampled (anti-aliasing),
masked by a heater-shield outline, with a dark contour and a light diagonal shading.

The output is deterministic: same data, same Pillow version, same bytes.
"""

from __future__ import annotations

import json
import math
import re
import unicodedata
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter

REPO_DIR = Path(__file__).resolve().parents[2]
FACTIONS_DIR = REPO_DIR / "data" / "factions"
HERALDRY_DIR = REPO_DIR / "game" / "assets" / "heraldry"

OUTPUT_SIZE = 128
SUPERSAMPLE = 4
CANVAS = OUTPUT_SIZE * SUPERSAMPLE

Color = tuple[int, int, int]

# Heraldic tinctures (French names, accents stripped) -> RGB.
TINCTURES: dict[str, Color] = {
    "or": (242, 194, 48),
    "argent": (245, 241, 230),
    "gueules": (176, 24, 43),
    "azur": (31, 58, 147),
    "sable": (26, 26, 26),
    "sinople": (30, 120, 60),
    "pourpre": (107, 42, 122),
}
ERMINE_FIELD: Color = (245, 241, 230)
OUTLINE: Color = (40, 28, 18)


def hex_to_rgb(value: str) -> Color:
    """Convert ``#RRGGBB`` to an RGB tuple."""
    value = value.lstrip("#")
    return (int(value[0:2], 16), int(value[2:4], 16), int(value[4:6], 16))


def _normalize(text: str) -> str:
    decomposed = unicodedata.normalize("NFD", text.lower())
    return "".join(char for char in decomposed if not unicodedata.combining(char))


@dataclass
class Blazon:
    """Simplified interpretation of a French blazon."""

    field: Color
    charge: Color
    text: str

    def has(self, *words: str) -> bool:
        """Whether any of ``words`` (accent-insensitive) appears in the blazon."""
        return any(_normalize(word) in self.text for word in words)

    def count(self, default: int) -> int:
        """Number of pieces named in the blazon (``quatre pals`` -> 4)."""
        numbers = {"deux": 2, "trois": 3, "quatre": 4, "cinq": 5, "six": 6}
        for word, value in numbers.items():
            if re.search(rf"\b{word}\b", self.text):
                return value
        return default

    def tincture_after(self, word: str) -> Color | None:
        """Tincture named right after ``word`` (``lion de pourpre`` -> purple)."""
        match = re.search(rf"{_normalize(word)}\w*\s+(?:d'|de\s+)(\w+)", self.text)
        if match and match.group(1) in TINCTURES:
            return TINCTURES[match.group(1)]
        return None


def parse_blazon(blazon: str, primary: str, secondary: str) -> Blazon:
    """Read the field tincture from the first words, charges default to ``secondary``."""
    text = _normalize(blazon)
    field = hex_to_rgb(primary)
    match = re.match(r"(?:d'|de\s+)(\w+)", text)
    if match and match.group(1) in TINCTURES:
        field = TINCTURES[match.group(1)]
    elif text.startswith("d'hermine"):
        field = ERMINE_FIELD
    return Blazon(field=field, charge=hex_to_rgb(secondary), text=text)


# --- Geometry helpers (normalized 0..1 coordinates on the canvas) ---------------


def _px(points: list[tuple[float, float]]) -> list[tuple[float, float]]:
    return [(x * CANVAS, y * CANVAS) for x, y in points]


def shield_polygon() -> list[tuple[float, float]]:
    """Heater shield outline: flat top, straight sides, two curves meeting at a point."""
    left, right, top, shoulder, bottom = 0.08, 0.92, 0.06, 0.5, 0.96
    control_y = 0.82
    steps = 24

    def bezier(start, control, end, t):
        return tuple(
            (1 - t) ** 2 * a + 2 * (1 - t) * t * b + t**2 * c
            for a, b, c in zip(start, control, end, strict=True)
        )

    point = (0.5, bottom)
    points = [(left, top), (right, top), (right, shoulder)]
    points += [
        bezier((right, shoulder), (right, control_y), point, i / steps)
        for i in range(1, steps + 1)
    ]
    points += [
        bezier(point, (left, control_y), (left, shoulder), i / steps)
        for i in range(1, steps + 1)
    ]
    return _px(points)


def _transform(
    shape: list[tuple[float, float]], cx: float, cy: float, scale: float
) -> list[tuple[float, float]]:
    """Place a shape defined in -0.5..0.5 space at (cx, cy) with ``scale``."""
    return _px([(cx + x * scale, cy + y * scale) for x, y in shape])


FLEUR_DE_LIS = [
    (0.0, -0.5),
    (0.1, -0.3),
    (0.08, -0.05),
    (0.2, -0.2),
    (0.38, -0.22),
    (0.45, -0.05),
    (0.35, 0.1),
    (0.25, 0.02),
    (0.28, 0.12),
    (0.4, 0.14),
    (0.4, 0.22),
    (0.1, 0.22),
    (0.06, 0.5),
    (-0.06, 0.5),
    (-0.1, 0.22),
    (-0.4, 0.22),
    (-0.4, 0.14),
    (-0.28, 0.12),
    (-0.25, 0.02),
    (-0.35, 0.1),
    (-0.45, -0.05),
    (-0.38, -0.22),
    (-0.2, -0.2),
    (-0.08, -0.05),
    (-0.1, -0.3),
]

# Stylized rampant lion facing dexter (viewer's left).
LION_RAMPANT = [
    (-0.18, -0.48),
    (0.0, -0.5),
    (0.08, -0.4),
    (0.02, -0.32),
    (0.12, -0.28),
    (0.08, -0.12),
    (0.2, -0.05),
    (0.3, -0.25),
    (0.45, -0.35),
    (0.4, -0.2),
    (0.3, 0.0),
    (0.22, 0.2),
    (0.3, 0.42),
    (0.14, 0.5),
    (0.08, 0.3),
    (-0.05, 0.28),
    (-0.12, 0.5),
    (-0.28, 0.5),
    (-0.16, 0.3),
    (-0.2, 0.1),
    (-0.38, 0.05),
    (-0.45, -0.1),
    (-0.3, -0.08),
    (-0.2, -0.2),
    (-0.4, -0.28),
    (-0.28, -0.36),
    (-0.14, -0.3),
    (-0.24, -0.4),
]

# Passant guardant lion ("léopard"): horizontal body, head turned to the viewer.
LEOPARD = [
    (-0.5, -0.1),
    (-0.4, -0.3),
    (-0.25, -0.3),
    (-0.22, -0.12),
    (0.25, -0.1),
    (0.4, -0.3),
    (0.5, -0.4),
    (0.46, -0.2),
    (0.35, -0.05),
    (0.38, 0.2),
    (0.3, 0.2),
    (0.25, 0.05),
    (0.15, 0.2),
    (0.08, 0.2),
    (0.1, 0.05),
    (-0.2, 0.05),
    (-0.22, 0.2),
    (-0.3, 0.2),
    (-0.3, 0.05),
    (-0.4, 0.2),
    (-0.46, 0.2),
    (-0.42, 0.0),
]

EAGLE = [
    (0.0, -0.42),
    (0.08, -0.36),
    (0.06, -0.22),
    (0.2, -0.3),
    (0.5, -0.4),
    (0.42, -0.18),
    (0.46, -0.1),
    (0.34, -0.02),
    (0.36, 0.06),
    (0.12, 0.08),
    (0.1, 0.22),
    (0.24, 0.42),
    (0.08, 0.34),
    (0.0, 0.48),
    (-0.08, 0.34),
    (-0.24, 0.42),
    (-0.1, 0.22),
    (-0.12, 0.08),
    (-0.36, 0.06),
    (-0.34, -0.02),
    (-0.46, -0.1),
    (-0.42, -0.18),
    (-0.5, -0.4),
    (-0.2, -0.3),
    (-0.06, -0.22),
    (-0.08, -0.36),
]

CASTLE = [
    (-0.4, 0.4),
    (-0.4, -0.2),
    (-0.45, -0.2),
    (-0.45, -0.4),
    (-0.3, -0.4),
    (-0.3, -0.3),
    (-0.18, -0.3),
    (-0.18, -0.5),
    (-0.05, -0.5),
    (-0.05, -0.4),
    (0.05, -0.4),
    (0.05, -0.5),
    (0.18, -0.5),
    (0.18, -0.3),
    (0.3, -0.3),
    (0.3, -0.4),
    (0.45, -0.4),
    (0.45, -0.2),
    (0.4, -0.2),
    (0.4, 0.4),
]


# --- Drawing -----------------------------------------------------------------------


def _draw_field(draw: ImageDraw.ImageDraw, blazon: Blazon) -> None:
    draw.rectangle((0, 0, CANVAS, CANVAS), fill=blazon.field)


def _draw_ermine(draw: ImageDraw.ImageDraw, color: Color) -> None:
    for row in range(7):
        for col in range(6):
            cx = 0.12 + col * 0.15 + (0.075 if row % 2 else 0.0)
            cy = 0.1 + row * 0.13
            spot = [
                (0, -0.5),
                (0.25, 0.3),
                (0.5, 0.5),
                (0, 0.35),
                (-0.5, 0.5),
                (-0.25, 0.3),
            ]
            draw.polygon(_transform(spot, cx, cy, 0.07), fill=color)
            for dx in (-0.018, 0.0, 0.018):
                x, y = (cx + dx) * CANVAS, (cy - 0.05) * CANVAS
                radius = 0.009 * CANVAS
                draw.ellipse(
                    (x - radius, y - radius, x + radius, y + radius), fill=color
                )


def _draw_semé(draw: ImageDraw.ImageDraw, color: Color) -> None:
    for row in range(6):
        for col in range(5):
            cx = 0.14 + col * 0.18 + (0.09 if row % 2 else 0.0)
            cy = 0.13 + row * 0.15
            draw.polygon(_transform(FLEUR_DE_LIS, cx, cy, 0.13), fill=color)


def _draw_pals(draw: ImageDraw.ImageDraw, color: Color, count: int) -> None:
    stripes = count * 2 + 1
    width = 0.84 / stripes
    for index in range(count):
        x0 = 0.08 + width * (2 * index + 1)
        draw.rectangle(_px([(x0, 0.0), (x0 + width, 1.0)]), fill=color)


def _draw_bends(draw: ImageDraw.ImageDraw, color: Color, pieces: int) -> None:
    width = 1.6 / pieces
    for index in range(1, pieces, 2):
        offset = -0.8 + index * width
        band = [
            (offset, 0.0),
            (offset + width, 0.0),
            (offset + width + 1.0, 1.0),
            (offset + 1.0, 1.0),
        ]
        draw.polygon(_px(band), fill=color)


def _draw_cross(draw: ImageDraw.ImageDraw, color: Color, arm: float = 0.16) -> None:
    draw.rectangle(_px([(0.5 - arm / 2, 0.0), (0.5 + arm / 2, 1.0)]), fill=color)
    draw.rectangle(_px([(0.0, 0.42 - arm / 2), (1.0, 0.42 + arm / 2)]), fill=color)


def _draw_fess(draw: ImageDraw.ImageDraw, color: Color) -> None:
    draw.rectangle(_px([(0.0, 0.34), (1.0, 0.56)]), fill=color)


def _draw_single_bend(draw: ImageDraw.ImageDraw, color: Color) -> None:
    draw.polygon(
        _px(
            [
                (0.0, -0.08),
                (0.2, -0.08),
                (1.0, 0.86),
                (1.0, 1.08),
                (0.8, 1.08),
                (0.0, 0.14),
            ]
        ),
        fill=color,
    )


def _draw_ladder(draw: ImageDraw.ImageDraw, color: Color, rungs: int = 4) -> None:
    for x0 in (0.34, 0.6):
        draw.rectangle(_px([(x0, 0.1), (x0 + 0.06, 0.86)]), fill=color)
    for index in range(rungs):
        y0 = 0.2 + index * 0.17
        draw.rectangle(_px([(0.34, y0), (0.66, y0 + 0.05)]), fill=color)


def _draw_label(draw: ImageDraw.ImageDraw, color: Color) -> None:
    draw.rectangle(_px([(0.12, 0.1), (0.88, 0.16)]), fill=color)
    for cx in (0.26, 0.5, 0.74):
        draw.polygon(
            _px(
                [
                    (cx - 0.05, 0.16),
                    (cx + 0.05, 0.16),
                    (cx + 0.07, 0.28),
                    (cx - 0.07, 0.28),
                ]
            ),
            fill=color,
        )


def _draw_crown(
    draw: ImageDraw.ImageDraw, color: Color, cx: float, cy: float, size: float
) -> None:
    half = size / 2
    points = [
        (cx - half, cy + half * 0.6),
        (cx - half, cy - half * 0.3),
        (cx - half * 0.5, cy + half * 0.05),
        (cx, cy - half * 0.6),
        (cx + half * 0.5, cy + half * 0.05),
        (cx + half, cy - half * 0.3),
        (cx + half, cy + half * 0.6),
    ]
    draw.polygon(_px(points), fill=color)


def _draw_crowns(draw: ImageDraw.ImageDraw, color: Color, count: int) -> None:
    spots = [(0.3, 0.26), (0.7, 0.26), (0.5, 0.6)] if count == 3 else [(0.5, 0.42)]
    for cx, cy in spots:
        _draw_crown(draw, color, cx, cy, 0.26)


def _draw_nettle(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """Holstein nettle leaf: a serrated escutcheon-shaped leaf."""
    outline = []
    teeth = 9
    for index in range(teeth + 1):
        t = index / teeth
        y = 0.14 + t * 0.62
        width = 0.34 * (1 - (t - 0.35) ** 2 * 1.4)
        tooth = 0.05 if index % 2 else 0.0
        outline.append((0.5 + width + tooth, y))
    outline.append((0.5, 0.86))
    for x, y in reversed(outline[:-1]):
        outline.append((1.0 - x, y))
    draw.polygon(_px(outline), fill=color)
    draw.rectangle(_px([(0.44, 0.38), (0.56, 0.56)]), fill=TINCTURES["gueules"])


def _draw_chains(draw: ImageDraw.ImageDraw, color: Color) -> None:
    width = int(0.035 * CANVAS)
    lines = [
        [(0.5, 0.06), (0.5, 0.96)],
        [(0.08, 0.42), (0.92, 0.42)],
        [(0.14, 0.1), (0.86, 0.82)],
        [(0.86, 0.1), (0.14, 0.82)],
    ]
    for line in lines:
        draw.line(_px(line), fill=color, width=width)
    orle = [
        (0.16, 0.13),
        (0.84, 0.13),
        (0.84, 0.52),
        (0.5, 0.86),
        (0.16, 0.52),
        (0.16, 0.13),
    ]
    draw.line(_px(orle), fill=color, width=width, joint="curve")
    for x, y in [(0.5, 0.25), (0.5, 0.62), (0.3, 0.42), (0.7, 0.42)]:
        radius = 0.03 * CANVAS
        draw.ellipse(
            (
                x * CANVAS - radius,
                y * CANVAS - radius,
                x * CANVAS + radius,
                y * CANVAS + radius,
            ),
            outline=color,
            width=width // 2,
        )
    cx, cy, radius = 0.5 * CANVAS, 0.42 * CANVAS, 0.06 * CANVAS
    draw.ellipse(
        (cx - radius, cy - radius, cx + radius, cy + radius), fill=(30, 140, 70)
    )


def _draw_keys(draw: ImageDraw.ImageDraw, first: Color, second: Color) -> None:
    width = int(0.05 * CANVAS)
    for color, (start, end) in (
        (first, ((0.22, 0.28), (0.78, 0.86))),
        (second, ((0.78, 0.28), (0.22, 0.86))),
    ):
        draw.line(_px([start, end]), fill=color, width=width)
        bx, by = end
        radius = 0.08 * CANVAS
        draw.ellipse(
            (
                bx * CANVAS - radius,
                by * CANVAS - radius,
                bx * CANVAS + radius,
                by * CANVAS + radius,
            ),
            outline=color,
            width=width,
        )
        sx, sy = start
        draw.rectangle(
            _px([(sx - 0.06, sy - 0.02), (sx + 0.06, sy + 0.06)]), fill=color
        )
    tiara = [(0.4, 0.24), (0.42, 0.1), (0.5, 0.04), (0.58, 0.1), (0.6, 0.24)]
    draw.polygon(_px(tiara), fill=TINCTURES["argent"], outline=TINCTURES["or"])
    for y in (0.12, 0.17, 0.22):
        draw.line(
            _px([(0.41, y), (0.59, y)]), fill=TINCTURES["or"], width=int(0.015 * CANVAS)
        )


def _draw_escutcheons(draw: ImageDraw.ImageDraw, color: Color) -> None:
    small = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.1), (0.0, 0.5), (-0.5, 0.1)]
    for cx, cy in [(0.5, 0.2), (0.28, 0.44), (0.5, 0.44), (0.72, 0.44), (0.5, 0.68)]:
        draw.polygon(_transform(small, cx, cy, 0.17), fill=color)
        for dx, dy in [
            (-0.03, -0.03),
            (0.03, -0.03),
            (0.0, 0.0),
            (-0.03, 0.03),
            (0.03, 0.03),
        ]:
            radius = 0.012 * CANVAS
            x, y = (cx + dx) * CANVAS, (cy + dy) * CANVAS
            draw.ellipse(
                (x - radius, y - radius, x + radius, y + radius),
                fill=TINCTURES["argent"],
            )


def _draw_guivre(draw: ImageDraw.ImageDraw, color: Color) -> None:
    points = []
    for index in range(40):
        t = index / 39
        points.append((0.5 + 0.18 * math.sin(t * math.pi * 3), 0.2 + 0.62 * t))
    draw.line(_px(points), fill=color, width=int(0.09 * CANVAS), joint="curve")
    head = [(0.38, 0.1), (0.62, 0.1), (0.66, 0.2), (0.5, 0.26), (0.34, 0.2)]
    draw.polygon(_px(head), fill=color)
    crown = [
        (0.4, 0.1),
        (0.42, 0.03),
        (0.46, 0.08),
        (0.5, 0.02),
        (0.54, 0.08),
        (0.58, 0.03),
        (0.6, 0.1),
    ]
    draw.polygon(_px(crown), fill=TINCTURES["or"])
    figure = [(0.44, 0.26), (0.56, 0.26), (0.6, 0.36), (0.4, 0.36)]
    draw.polygon(_px(figure), fill=TINCTURES["gueules"])


def _draw_quarterly(image: Image.Image, blazon: Blazon) -> None:
    draw = ImageDraw.Draw(image)
    second_field = TINCTURES["argent"] if blazon.has("argent") else blazon.charge
    lion_color = blazon.tincture_after("lion") or blazon.field
    for quarter in range(4):
        col, row = quarter % 2, quarter // 2
        box = (col * 0.5, row * 0.5 - 0.04, col * 0.5 + 0.5, row * 0.5 + 0.46)
        primary_quarter = quarter in (0, 3)
        fill = blazon.field if primary_quarter else second_field
        draw.rectangle(
            _px(
                [(box[0], box[1] if row else 0.0), (box[2], box[3] if not row else 1.0)]
            ),
            fill=fill,
        )
        cx, cy = col * 0.5 + 0.27, row * 0.46 + 0.26
        if primary_quarter:
            charge = (
                LION_RAMPANT
                if blazon.has("lion") and not blazon.has("chateau")
                else CASTLE
            )
            draw.polygon(_transform(charge, cx, cy, 0.28), fill=blazon.charge)
        else:
            charge = LION_RAMPANT if blazon.has("lion") else FLEUR_DE_LIS
            draw.polygon(_transform(charge, cx, cy, 0.3), fill=lion_color)


def _draw_charges(image: Image.Image, blazon: Blazon) -> None:
    draw = ImageDraw.Draw(image)
    charge = blazon.charge
    if blazon.has("ecartele"):
        _draw_quarterly(image, blazon)
    elif blazon.has("hermine"):
        _draw_ermine(draw, TINCTURES["sable"])
    elif blazon.has("seme"):
        _draw_semé(draw, charge)
        if blazon.has("lambel"):
            _draw_label(draw, blazon.tincture_after("lambel") or TINCTURES["gueules"])
    elif blazon.has("a la bande"):
        _draw_single_bend(draw, charge)
    elif blazon.has("bande"):
        _draw_bends(draw, charge, blazon.count(6))
    elif blazon.has("fasce"):
        _draw_fess(draw, charge)
    elif blazon.has("echelle"):
        _draw_ladder(draw, charge)
    elif blazon.has("couronnes"):
        _draw_crowns(draw, charge, blazon.count(1))
    elif blazon.has("ortie"):
        _draw_nettle(draw, charge)
    elif blazon.has("lis florence"):
        draw.polygon(_transform(FLEUR_DE_LIS, 0.5, 0.44, 0.62), fill=charge)
    elif blazon.has("pals", "pal "):
        _draw_pals(draw, charge, blazon.count(1))
    elif blazon.has("chaines"):
        _draw_chains(draw, charge)
    elif blazon.has("clefs"):
        _draw_keys(draw, TINCTURES["argent"], TINCTURES["or"])
    elif blazon.has("ecussons"):
        _draw_escutcheons(draw, TINCTURES["azur"])
    elif blazon.has("guivre"):
        _draw_guivre(draw, charge)
    elif blazon.has("aigle"):
        draw.polygon(_transform(EAGLE, 0.5, 0.46, 0.72), fill=charge)
    elif blazon.has("leopard"):
        count = blazon.count(3)
        for index in range(count):
            draw.polygon(
                _transform(LEOPARD, 0.5, 0.2 + index * 0.24, 0.5 - index * 0.06),
                fill=charge,
            )
    elif blazon.has("lion"):
        draw.polygon(_transform(LION_RAMPANT, 0.5, 0.46, 0.62), fill=charge)
    elif blazon.has("croix alesee"):
        draw.rectangle(_px([(0.42, 0.18), (0.58, 0.7)]), fill=charge)
        draw.rectangle(_px([(0.24, 0.36), (0.76, 0.52)]), fill=charge)
    elif blazon.has("croix"):
        _draw_cross(draw, charge)

    if blazon.has("trescheur"):
        width = int(0.018 * CANVAS)
        for inset in (0.14, 0.18):
            outline = [
                (inset, inset),
                (1 - inset, inset),
                (1 - inset, 0.5),
                (0.5, 0.94 - inset),
                (inset, 0.5),
                (inset, inset),
            ]
            draw.line(_px(outline), fill=charge, width=width, joint="curve")
    if blazon.has("bordure"):
        border_color = blazon.tincture_after("bordure") or TINCTURES["gueules"]
        _draw_border(image, border_color, castles=blazon.has("chateaux"))


def _draw_border(image: Image.Image, color: Color, castles: bool) -> None:
    mask = Image.new("L", image.size, 0)
    mask_draw = ImageDraw.Draw(mask)
    mask_draw.polygon(shield_polygon(), fill=255)
    inner = mask.filter(ImageFilter.MinFilter(int(0.09 * CANVAS) | 1))
    ring = ImageChops.subtract(mask, inner)
    image.paste(Image.new("RGB", image.size, color), mask=ring)
    if castles:
        draw = ImageDraw.Draw(image)
        for cx, cy in [
            (0.13, 0.15),
            (0.13, 0.38),
            (0.87, 0.15),
            (0.87, 0.38),
            (0.5, 0.1),
            (0.22, 0.72),
            (0.78, 0.72),
        ]:
            draw.polygon(_transform(CASTLE, cx, cy, 0.06), fill=TINCTURES["or"])


def render_shield(blazon: Blazon) -> Image.Image:
    """Render a shield for ``blazon`` as a 128x128 RGBA image."""
    field = Image.new("RGB", (CANVAS, CANVAS))
    _draw_field(ImageDraw.Draw(field), blazon)
    _draw_charges(field, blazon)

    # Light diagonal shading: brighter at the top-left, darker at the bottom-right.
    gradient = (
        Image.linear_gradient("L").rotate(45, expand=False).resize((CANVAS, CANVAS))
    )
    shade = Image.merge("RGB", (gradient, gradient, gradient))
    field = Image.blend(field, ImageChops.multiply(field, shade), 0.18)
    highlight = Image.new("RGB", (CANVAS, CANVAS), (255, 255, 255))
    field = Image.composite(
        highlight, field, ImageChops.invert(gradient).point(lambda v: v // 10)
    )

    mask = Image.new("L", (CANVAS, CANVAS), 0)
    ImageDraw.Draw(mask).polygon(shield_polygon(), fill=255)
    shield = Image.new("RGBA", (CANVAS, CANVAS), (0, 0, 0, 0))
    shield.paste(field, mask=mask)
    ImageDraw.Draw(shield).line(
        [*shield_polygon(), shield_polygon()[0]],
        fill=(*OUTLINE, 255),
        width=int(0.03 * CANVAS),
        joint="curve",
    )
    return shield.resize((OUTPUT_SIZE, OUTPUT_SIZE), Image.Resampling.LANCZOS)


def load_factions(factions_dir: Path = FACTIONS_DIR) -> list[dict]:
    """Load every faction JSON, sorted by id."""
    factions = [
        json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(factions_dir.glob("*.json"))
    ]
    return sorted(factions, key=lambda faction: faction["id"])


def shield_for_faction(faction: dict) -> Image.Image:
    """Render the shield of one faction dictionary."""
    heraldry = faction["heraldry"]
    blazon = parse_blazon(
        heraldry.get("blazon", ""),
        heraldry["primary_color"],
        heraldry["secondary_color"],
    )
    return render_shield(blazon)


def build(
    factions_dir: Path = FACTIONS_DIR, out_dir: Path = HERALDRY_DIR
) -> list[Path]:
    """Write ``<out_dir>/<faction id>.png`` for every faction and return the paths."""
    out_dir.mkdir(parents=True, exist_ok=True)
    written = []
    for faction in load_factions(factions_dir):
        path = out_dir / f"{faction['id']}.png"
        shield_for_faction(faction).save(path, optimize=True)
        written.append(path)
    return written
