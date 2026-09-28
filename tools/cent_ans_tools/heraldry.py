"""Procedural heraldry: one 128x128 shield PNG per faction, drawn with Pillow.

The blazon (``heraldry.blazon`` in ``data/factions/*.json``) is interpreted with a
small keyword grammar: the field tincture comes from the first words (``D'or``,
``De gueules``...), falling back to ``primary_color``; charges (``semé de fleurs de
lis``, ``écartelé``, ``lion``, ``léopards``, ``aigle``, ``pals``, ``bandé``, ``croix``,
``chaînes``, ``clefs``, ``écussons``, ``guivre``, ``hermine``, ``bordure``,
``trescheur``) are drawn in ``secondary_color`` unless the blazon names another
tincture. Everything is drawn at 4x resolution then downsampled (anti-aliasing),
masked by a heater-shield outline, with a dark contour and a light diagonal shading.

House arms (lot DA1, ``data/heraldry/houses.json`` -> ``houses/<id>.png``) use a
richer grammar (v2, :func:`render_house_field`): every tincture is read from the
blazon itself, the field may be divided (``bandé``, ``burelé``, ``échiqueté``,
``fuselé``, ``coupé``, ``écartelé en sautoir``, ``semé``), ``écartelé : aux 1 et 4 …
; aux 2 et 3 …`` recurses, and ordinaries and charges are drawn in the order they
are named (number = word before, tincture = first tincture after, ``chargé de`` =
on the previous piece, ``accompagnée de`` = around the bend). The faction shields
keep the original grammar so that they stay byte-identical.

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

from cent_ans_tools import heraldic_charges

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


def _draw_saltire(draw: ImageDraw.ImageDraw, color: Color, arm: float = 0.16) -> None:
    """Diagonal cross (X), the saltire of the FitzGerald arms (FE4c)."""
    width = int(arm * CANVAS)
    draw.line(_px([(0.0, 0.0), (1.0, 1.0)]), fill=color, width=width)
    draw.line(_px([(1.0, 0.0), (0.0, 1.0)]), fill=color, width=width)


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


def _draw_wheel(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """A cartwheel with six spokes (FE4b: Mainzer Rad)."""
    cx, cy, radius, inner = 0.5, 0.47, 0.32, 0.06
    width = int(0.05 * CANVAS)
    draw.ellipse(
        _px([(cx - radius, cy - radius), (cx + radius, cy + radius)]),
        outline=color,
        width=width,
    )
    draw.ellipse(_px([(cx - inner, cy - inner), (cx + inner, cy + inner)]), fill=color)
    for angle in range(0, 360, 60):
        rad = math.radians(angle)
        start = (cx + inner * math.cos(rad), cy + inner * math.sin(rad))
        end = (cx + radius * math.cos(rad), cy + radius * math.sin(rad))
        draw.line(_px([start, end]), fill=color, width=width)


def _draw_column(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """A single standing column (FE4b: perron/colonne of Liège)."""
    draw.rectangle(_px([(0.42, 0.08), (0.58, 0.84)]), fill=color)
    draw.rectangle(_px([(0.3, 0.02), (0.7, 0.1)]), fill=color)
    draw.rectangle(_px([(0.3, 0.82), (0.7, 0.9)]), fill=color)


def _draw_escutcheon_single(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """One inescutcheon at fess point (FE4b: Clèves, singular, unlike Portugal's five)."""
    small = [(-0.5, -0.5), (0.5, -0.5), (0.5, 0.1), (0.0, 0.5), (-0.5, 0.1)]
    draw.polygon(_transform(small, 0.5, 0.47, 0.34), fill=color)


def _draw_chevrons(draw: ImageDraw.ImageDraw, color: Color, count: int) -> None:
    """Stacked chevrons (FE4b: la Marck)."""
    for index in range(count):
        y0 = 0.14 + index * 0.24
        draw.polygon(
            _px(
                [
                    (0.1, y0 + 0.2),
                    (0.5, y0),
                    (0.9, y0 + 0.2),
                    (0.9, y0 + 0.34),
                    (0.5, y0 + 0.14),
                    (0.1, y0 + 0.34),
                ]
            ),
            fill=color,
        )


def _draw_antlers(draw: ImageDraw.ImageDraw, color: Color, count: int = 3) -> None:
    """Stacked stylised antlers (FE4b: Wurtemberg, bois de cerf)."""
    width = int(0.035 * CANVAS)
    for index in range(count):
        cx, cy = 0.5, 0.16 + index * 0.28
        for side in (-1, 1):
            draw.line(
                _px([(cx, cy + 0.1), (cx + side * 0.18, cy - 0.08)]),
                fill=color,
                width=width,
            )
            draw.line(
                _px([(cx + side * 0.09, cy - 0.02), (cx + side * 0.26, cy - 0.18)]),
                fill=color,
                width=int(width * 0.7),
            )


def _draw_buffalo_head(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """A stylised buffalo head (FE4b: Mecklembourg, tête de buffle)."""
    cx, cy = 0.5, 0.5
    draw.ellipse(_px([(cx - 0.22, cy - 0.2), (cx + 0.22, cy + 0.24)]), fill=color)
    width = int(0.045 * CANVAS)
    draw.line(
        _px([(cx - 0.2, cy - 0.1), (cx - 0.4, cy - 0.32)]), fill=color, width=width
    )
    draw.line(
        _px([(cx + 0.2, cy - 0.1), (cx + 0.4, cy - 0.32)]), fill=color, width=width
    )


def _draw_key_single(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """One key in pale, bow (anneau) toward the chief (FE: Bremen)."""
    width = int(0.05 * CANVAS)
    draw.line(_px([(0.5, 0.22), (0.5, 0.82)]), fill=color, width=width)
    radius = 0.1 * CANVAS
    cx, cy = 0.5 * CANVAS, 0.18 * CANVAS
    draw.ellipse(
        (cx - radius, cy - radius, cx + radius, cy + radius), outline=color, width=width
    )
    for y0 in (0.66, 0.76):
        draw.rectangle(_px([(0.5, y0), (0.66, y0 + 0.06)]), fill=color)


def _draw_cauldrons(image: Image.Image, checky: Image.Image, count: int) -> None:
    """Cauldrons (chaudières) checky, stacked in pale (FE: Lara)."""
    spots = [0.28, 0.68][:count] if count <= 2 else [0.2, 0.5, 0.8][:count]
    for cy in spots:
        body = [
            (-0.22, -0.16),
            (0.22, -0.16),
            (0.28, 0.1),
            (0.16, 0.24),
            (-0.16, 0.24),
            (-0.28, 0.1),
        ]
        mask, mask_draw = _new_mask()
        mask_draw.polygon(_transform(body, 0.5, cy, 0.6), fill=255)
        width = int(0.03 * CANVAS)
        for side in (-1, 1):
            mask_draw.arc(
                _px(
                    [
                        (0.5 + side * 0.36 - 0.12, cy - 0.2),
                        (0.5 + side * 0.36 + 0.12, cy - 0.02),
                    ]
                ),
                start=200 if side < 0 else -20,
                end=340 if side < 0 else 120,
                fill=255,
                width=width,
            )
        _fill_mask_with_paint(image, mask, checky)


def _fill_mask_with_paint(
    image: Image.Image, mask: Image.Image, paint: Color | Image.Image
) -> None:
    source = paint if isinstance(paint, Image.Image) else _solid_rgb(paint)
    image.paste(source, mask=mask)


def _draw_hand(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """The "red hand" of Ulster: a dexter hand couped at the wrist (FE: Tyrone)."""
    hand = [
        (-0.16, 0.4),
        (-0.16, -0.06),
        (-0.24, -0.14),
        (-0.24, -0.3),
        (-0.16, -0.3),
        (-0.16, -0.18),
        (-0.1, -0.18),
        (-0.1, -0.4),
        (-0.02, -0.4),
        (-0.02, -0.2),
        (0.04, -0.2),
        (0.04, -0.42),
        (0.12, -0.42),
        (0.12, -0.2),
        (0.18, -0.2),
        (0.18, -0.34),
        (0.26, -0.34),
        (0.26, -0.1),
        (0.16, 0.04),
        (0.16, 0.4),
    ]
    draw.polygon(_transform(hand, 0.5, 0.5, 1.0), fill=color)


def _draw_ship_shape(
    draw: ImageDraw.ImageDraw,
    cx: float,
    cy: float,
    scale: float,
    fill: int | Color = 255,
) -> None:
    """A lymphad (galley), sail furled, centred at ``(cx, cy)`` in a ``scale`` box."""
    hull = [
        (-0.42, 0.14),
        (0.42, 0.14),
        (0.3, 0.32),
        (-0.3, 0.32),
    ]
    draw.polygon(_transform(hull, cx, cy, scale), fill=fill)
    width = max(1, int(0.03 * scale * CANVAS))
    draw.line(
        _transform([(0.0, 0.14), (0.0, -0.34)], cx, cy, scale), fill=fill, width=width
    )
    draw.line(
        _transform([(0.0, -0.28), (0.18, -0.2)], cx, cy, scale), fill=fill, width=width
    )
    draw.line(
        _transform([(0.0, -0.34), (-0.18, -0.24), (0.0, -0.18)], cx, cy, scale),
        fill=fill,
        width=width,
    )
    for x in (-0.3, 0.0, 0.3):
        draw.line(
            _transform([(x * 0.5, 0.14), (x, 0.02)], cx, cy, scale),
            fill=fill,
            width=width,
        )


def _draw_ship(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """A lymphad (galley), sail furled (FE: the Isles)."""
    _draw_ship_shape(draw, 0.5, 0.5, 1.0, fill=color)


def _draw_crescent(image: Image.Image, color: Color) -> None:
    """Crescent decrescent (horns to sinister), FE: Luna."""
    radius = 0.34 * CANVAS
    cx, cy = 0.5 * CANVAS, 0.5 * CANVAS
    mask, mask_draw = _new_mask()
    mask_draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=255)
    inner_radius = 0.3 * CANVAS
    inner_cx = cx + 0.16 * CANVAS
    mask_draw.ellipse(
        (
            inner_cx - inner_radius,
            cy - inner_radius,
            inner_cx + inner_radius,
            cy + inner_radius,
        ),
        fill=0,
    )
    _fill_mask_with_paint(image, mask, color)


def _draw_ox(draw: ImageDraw.ImageDraw, color: Color) -> None:
    """A standing ox, simplified silhouette (FE: Urgell)."""
    body = [
        (-0.38, 0.1),
        (-0.4, -0.06),
        (-0.3, -0.18),
        (-0.1, -0.22),
        (0.16, -0.2),
        (0.3, -0.06),
        (0.34, 0.1),
        (0.26, 0.1),
        (0.26, 0.3),
        (0.18, 0.3),
        (0.18, 0.12),
        (-0.02, 0.12),
        (-0.02, 0.3),
        (-0.1, 0.3),
        (-0.1, 0.12),
        (-0.3, 0.1),
        (-0.3, 0.3),
        (-0.38, 0.3),
    ]
    draw.polygon(_transform(body, 0.5, 0.5, 1.0), fill=color)
    horns = [(-0.36, -0.2), (-0.48, -0.34), (-0.4, -0.16)]
    draw.polygon(_transform(horns, 0.5, 0.5, 1.0), fill=color)
    horns2 = [(-0.16, -0.24), (-0.1, -0.4), (-0.02, -0.26)]
    draw.polygon(_transform(horns2, 0.5, 0.5, 1.0), fill=color)


def _draw_chief(draw: ImageDraw.ImageDraw, color: Color, denched: bool) -> None:
    """A chief (chef), optionally with a dancetty (denché) lower edge (FE: Ormond)."""
    if not denched:
        draw.rectangle(_px([(0.0, 0.0), (1.0, 0.26)]), fill=color)
        return
    teeth = 6
    points = [(0.0, 0.0), (1.0, 0.0)]
    bottom = []
    for index in range(teeth + 1):
        x = index / teeth
        y = 0.32 if index % 2 == 0 else 0.2
        bottom.append((x, y))
    points.append((1.0, bottom[-1][1]))
    points += list(reversed(bottom))
    draw.polygon(_px(points), fill=color)


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


# --- Vector charges (lot DA1b) -------------------------------------------------------

_TINCTURE_NAMES = "or|argent|gueules|azur|sable|sinople|pourpre"
_ARMED_RE = re.compile(
    r"\b(?:arm|lampass|becqu|membr|peautr|lore|barbe|crete)\w*[\w ,']*?"
    rf"\s(?:d'|de |du )({_TINCTURE_NAMES})\b"
)
_CROWN_RE = re.compile(rf"\bcouronn\w*[\w ,']*?\s(?:d'|de |du )({_TINCTURE_NAMES})\b")
# Details of a sable charge are drawn in a light line, as in the armorials.
SABLE_OUTLINE: Color = (128, 120, 108)


def armed_tincture(text: str, body: Paint, field: Paint | None = None) -> Color:
    """Claws, tongue, beak (``armé, lampassé, becqué...``); gueules or azur by default."""
    match = _ARMED_RE.search(_normalize(text))
    if match:
        return TINCTURES[match[1]]
    red = TINCTURES["gueules"]
    return TINCTURES["azur"] if red in (body, field) else red


def crown_tincture(text: str) -> Color:
    """Tincture of the crown (``couronné d'or``), or by default."""
    match = _CROWN_RE.search(_normalize(text))
    return TINCTURES[match[1]] if match else TINCTURES["or"]


def lion_variant(text: str) -> tuple[str, bool]:
    """Drawing of a lion named in ``text`` and whether a crown must be added on it."""
    text = _normalize(text)
    crowned = "couronn" in text
    if "saint marc" in text or "aile" in text.split():
        return "lion_aile", False
    if "queue fourchee" in text:
        crossed = "sautoir" in text or "passee en" in text
        return (
            "lion_queue_fourchee_sautoir" if crossed else "lion_queue_fourchee"
        ), crowned
    if crowned:
        return "lion_couronne", False
    return "lion", False


def charge_paints(
    charge_id: str,
    body: Paint,
    text: str = "",
    field: Paint | None = None,
) -> dict[str, Paint | None]:
    """Paint of every role of a charge (None = left unpainted, the field shows through)."""
    sable = body == TINCTURES["sable"]
    paints: dict[str, Paint | None] = {
        "body": body,
        "armed": armed_tincture(text, body, field),
        "crown": crown_tincture(text),
        "openings": None,
        "issant": TINCTURES["gueules"],
        "argent": TINCTURES["argent"],
        "outline": SABLE_OUTLINE if sable else OUTLINE,
    }
    if isinstance(body, Image.Image):
        paints["shade"] = body
    else:
        paints["shade"] = tuple(int(c * 0.72) for c in body)
    if charge_id == "chateau":
        match = re.search(
            rf"(?:ouvert|ajoure|maconne)\w*[\w ,']*?\s(?:d'|de |du )({_TINCTURE_NAMES})\b",
            _normalize(text),
        )
        paints["openings"] = TINCTURES[match[1]] if match else None
    return paints


def paint_charge(
    image: Image.Image,
    charge_id: str,
    spots: list[tuple[float, float]],
    scale: float,
    paints: dict[str, Paint | None],
    crowned: bool = False,
    height: float | None = None,
) -> None:
    """Paint a vector charge centred on each spot, fitted in a ``scale`` square.

    ``height`` (canvas fraction) overrides the height of the box, for wide charges such as
    the leopard. ``crowned`` adds the crown drawing on a charge drawn bareheaded.
    """
    box = (int(scale * CANVAS), int((height or scale) * CANVAS))
    boost = 3 if max(box) >= 96 else 0
    masks, size = heraldic_charges.fitted_masks(charge_id, box, outline_boost=boost)
    anchor = heraldic_charges.crown_anchor(charge_id) if crowned else None
    crown_masks, crown_size = ({}, (0, 0))
    if anchor:
        crown_box = (int(anchor[2] * size[0]),) * 2
        # No thickened outline on the small crown: it would swallow its gold.
        crown_masks, crown_size = heraldic_charges.fitted_masks("couronne", crown_box)
    base: dict[str, Image.Image] = {}
    crown: dict[str, Image.Image] = {}
    for cx, cy in spots:
        left = round(cx * CANVAS - size[0] / 2)
        top = round(cy * CANVAS - size[1] / 2)
        _accumulate(base, masks, (left, top))
        if anchor:
            crown_left = round(left + anchor[0] * size[0] - crown_size[0] / 2)
            crown_top = round(top + anchor[1] * size[1] - crown_size[1] / 2)
            _accumulate(crown, crown_masks, (crown_left, crown_top))
    if crown:
        # The added crown hides the lines of the head under it.
        cover = _combine(crown.values())
        base = {role: ImageChops.subtract(mask, cover) for role, mask in base.items()}
    for layers in (base, crown):
        for role in heraldic_charges.ROLE_ORDER:
            paint = paints.get(role)
            if role in layers and paint is not None:
                source = paint if isinstance(paint, Image.Image) else _solid_rgb(paint)
                image.paste(source, mask=layers[role])


def _accumulate(
    layers: dict[str, Image.Image],
    masks: dict[str, Image.Image],
    offset: tuple[int, int],
) -> None:
    """Add ``masks`` placed at ``offset`` to the full-canvas ``layers`` (max blend)."""
    for role, mask in masks.items():
        if role not in layers:
            layers[role] = Image.new("L", (CANVAS, CANVAS), 0)
        box = _box_of(offset, mask)
        layers[role].paste(ImageChops.lighter(layers[role].crop(box), mask), offset)


def _box_of(offset: tuple[int, int], mask: Image.Image) -> tuple[int, int, int, int]:
    return (offset[0], offset[1], offset[0] + mask.width, offset[1] + mask.height)


def _solid_rgb(color: Color) -> Image.Image:
    return Image.new("RGB", (CANVAS, CANVAS), color)


def _draw_quarterly(image: Image.Image, blazon: Blazon) -> None:
    draw = ImageDraw.Draw(image)
    second_field = TINCTURES["argent"] if blazon.has("argent") else blazon.charge
    lion_color = blazon.tincture_after("lion") or blazon.field
    lion_text = blazon.text.split("lion", 1)[-1]
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
            castle = not (blazon.has("lion") and not blazon.has("chateau"))
            color, text = blazon.charge, blazon.text
            charge, crowned = ("chateau", False) if castle else lion_variant(lion_text)
            size = 0.3
        elif blazon.has("lion"):
            color, text = lion_color, lion_text
            charge, crowned = lion_variant(lion_text)
            size = 0.32
        else:
            draw.polygon(_transform(FLEUR_DE_LIS, cx, cy, 0.3), fill=lion_color)
            continue
        paints = charge_paints(charge, color, text, fill)
        paint_charge(image, charge, [(cx, cy)], size, paints, crowned=crowned)


def _draw_charges(image: Image.Image, blazon: Blazon) -> None:
    draw = ImageDraw.Draw(image)
    charge = blazon.charge
    if blazon.has("ecartele"):
        _draw_quarterly(image, blazon)
    elif blazon.has("hermine"):
        _draw_ermine(draw, TINCTURES["sable"])
        if blazon.has("lambel"):
            _draw_label(draw, blazon.tincture_after("lambel") or TINCTURES["gueules"])
        if blazon.has("sautoir"):
            _draw_saltire(draw, charge)
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
    elif blazon.has("chaudiere"):
        checky = checky_pattern(TINCTURES["or"], TINCTURES["gueules"], 0.12)
        _draw_cauldrons(image, checky, blazon.count(2))
    elif blazon.has("clef"):
        _draw_key_single(draw, charge)
    elif blazon.has("ecussons"):
        _draw_escutcheons(draw, TINCTURES["azur"])
    elif blazon.has("ecusson"):
        _draw_escutcheon_single(draw, charge)
    elif blazon.has("roue"):
        _draw_wheel(draw, charge)
    elif blazon.has("colonne"):
        _draw_column(draw, charge)
    elif blazon.has("chevron"):
        _draw_chevrons(draw, charge, blazon.count(3))
    elif blazon.has("bois de cerf"):
        _draw_antlers(draw, charge, blazon.count(3))
    elif blazon.has("tete de buffle"):
        _draw_buffalo_head(draw, charge)
    elif blazon.has("main"):
        _draw_hand(draw, charge)
    elif blazon.has("nef"):
        _draw_ship(draw, charge)
    elif blazon.has("lune"):
        _draw_crescent(image, charge)
    elif blazon.has("bœuf"):
        _draw_ox(draw, charge)
    elif blazon.has("chef"):
        _draw_chief(draw, charge, denched=blazon.has("denche"))
    elif blazon.has("guivre"):
        paints = charge_paints("guivre", charge, blazon.text, blazon.field)
        paint_charge(image, "guivre", [(0.5, 0.47)], 0.8, paints)
    elif blazon.has("aigle"):
        paints = charge_paints("aigle", charge, blazon.text, blazon.field)
        paint_charge(image, "aigle", [(0.5, 0.47)], 0.8, paints)
    elif blazon.has("leopard"):
        paints = charge_paints("leopard", charge, blazon.text, blazon.field)
        for index in range(blazon.count(3)):
            width = 0.7 - index * 0.06
            spot = [(0.5, 0.2 + index * 0.25)]
            paint_charge(image, "leopard", spot, width, paints, height=0.24)
    elif blazon.has("lion"):
        lion, crowned = lion_variant(blazon.text)
        paints = charge_paints(lion, charge, blazon.text, blazon.field)
        size = 0.6 if blazon.has("trescheur", "bordure") else 0.74
        paint_charge(image, lion, [(0.5, 0.47)], size, paints, crowned=crowned)
    elif blazon.has("croix alesee"):
        draw.rectangle(_px([(0.42, 0.18), (0.58, 0.7)]), fill=charge)
        draw.rectangle(_px([(0.24, 0.36), (0.76, 0.52)]), fill=charge)
    elif blazon.has("croix"):
        _draw_cross(draw, charge)
    elif blazon.has("sautoir"):
        # Last: "en sautoir" also describes other charges (Navarre's chains, the keys).
        _draw_saltire(draw, charge)

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
    return finish_shield(field)


def finish_shield(field: Image.Image) -> Image.Image:
    """Shade, mask and outline a CANVAS × CANVAS RGB field into a 128x128 RGBA shield."""
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
    """Render the shield of one faction dictionary.

    Quarterly blazons (Castile, Hainaut) go through the grammar v2, which reads the
    tinctures of each quarter; the others keep the original faction grammar.
    """
    heraldry = faction["heraldry"]
    if is_quarterly(heraldry.get("blazon", "")):
        return finish_shield(render_house_field(heraldry["blazon"]))
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


# --- House arms: grammar v2 (lot DA1) ----------------------------------------------

HOUSES_FILE = REPO_DIR / "data" / "heraldry" / "houses.json"
HOUSES_DIR = HERALDRY_DIR / "houses"

# A paint is a plain tincture or a full-canvas pattern image (fur, checky, compony...).
Paint = Color | Image.Image

NUMBERS = {
    "un": 1,
    "une": 1,
    "deux": 2,
    "trois": 3,
    "quatre": 4,
    "cinq": 5,
    "six": 6,
    "sept": 7,
    "huit": 8,
    "neuf": 9,
    "dix": 10,
}
_TINCTURE_WORDS = "or|argent|gueules|azur|sable|sinople|pourpre|hermine|vair"
_OF = r"(?:d'|de |du )"
_TINCTURE_RE = re.compile(rf"{_OF}({_TINCTURE_WORDS})\b")
_PAIR_RE = re.compile(
    rf"(?:compone|echiquetee?)\s+{_OF}({_TINCTURE_WORDS})\s+et\s+{_OF}({_TINCTURE_WORDS})\b"
)

# Keyword (accent-free) -> piece kind. Longest keywords are matched first.
PIECES: dict[str, str] = {
    "franc-quartier": "franc_quartier",
    "franc quartier": "franc_quartier",
    "trescheur": "trescheur",
    "fleurs de lis": "lis",
    "fleur de lis": "lis",
    "lionceaux": "lionceau",
    "lion": "lion",
    "leopards": "leopard",
    "leopard": "leopard",
    "aigle": "aigle",
    "chateaux": "chateau",
    "chateau": "chateau",
    "etoiles": "etoile",
    "etoile": "etoile",
    "roses": "rose",
    "rose": "rose",
    "tourteaux": "tourteau",
    "besants": "tourteau",
    "coussins": "coussin",
    "fusees": "fusee",
    "dauphin": "dauphin",
    "gonfanon": "gonfanon",
    "nef": "nef",
    "perron": "perron",
    "chevrons": "chevron",
    "chevron": "chevron",
    "fasces": "fasce",
    "fasce": "fasce",
    "pals": "pal",
    "pal": "pal",
    "bandes": "bande",
    "bande": "bande",
    "barres": "barre",
    "barre": "barre",
    "baton": "baton",
    "cotices": "cotice",
    "croix": "croix",
    "sautoir": "sautoir",
    "chef": "chef",
    "bordure": "bordure",
    "orle": "orle",
    "lambel": "lambel",
}
_PIECE_RE = re.compile(
    r"\b(" + "|".join(sorted(map(re.escape, PIECES), key=len, reverse=True)) + r")\b"
)
# Plural keywords without a number word.
PLURAL_DEFAULT = 3

STAR = [
    (
        0.5 * math.sin(i * math.pi / 5) * (1.0 if i % 2 == 0 else 0.42),
        -0.5 * math.cos(i * math.pi / 5) * (1.0 if i % 2 == 0 else 0.42),
    )
    for i in range(10)
]
LOZENGE = [(0.0, -0.5), (0.3, 0.0), (0.0, 0.5), (-0.3, 0.0)]
# Cushion set lozengewise (Randolph), tassels added at its four points.
CUSHION = [(0.0, -0.4), (0.3, 0.0), (0.0, 0.4), (-0.3, 0.0)]
# Heraldic dolphin embowed, facing dexter: spine of a thick curved body (the mask
# draws it as a line of decreasing width), head, tail and fins in the second tincture.
DOLPHIN_SPINE = [
    (-0.22, -0.30), (-0.02, -0.36), (0.18, -0.30), (0.30, -0.12),
    (0.28, 0.10), (0.16, 0.28), (0.00, 0.36),
]  # fmt: skip
DOLPHIN_TAIL = [(0.02, 0.34), (-0.20, 0.52), (-0.10, 0.36), (-0.22, 0.24), (0.04, 0.28)]
DOLPHIN_FINS = [
    [(0.02, -0.38), (0.16, -0.54), (0.22, -0.32)],
    [(-0.08, -0.24), (-0.18, -0.04), (0.02, -0.2)],
]
GONFANON = [
    (-0.4, -0.36), (0.4, -0.36), (0.4, 0.12), (0.3, 0.42), (0.2, 0.12),
    (0.1, 0.12), (0.0, 0.42), (-0.1, 0.12), (-0.2, 0.12), (-0.3, 0.42), (-0.4, 0.12),
]  # fmt: skip
SHAPES: dict[str, list[tuple[float, float]]] = {
    "lion": LION_RAMPANT,
    "lionceau": LION_RAMPANT,
    "leopard": LEOPARD,
    "aigle": EAGLE,
    "lis": FLEUR_DE_LIS,
    "chateau": CASTLE,
    "etoile": STAR,
    "fusee": LOZENGE,
    "coussin": CUSHION,
    "gonfanon": GONFANON,
}
ORDINARIES = {
    "franc_quartier",
    "trescheur",
    "chevron",
    "fasce",
    "pal",
    "bande",
    "barre",
    "baton",
    "cotice",
    "croix",
    "sautoir",
    "chef",
    "bordure",
    "orle",
    "lambel",
}
Region = tuple[float, float, float, float]


@dataclass
class Piece:
    """One ordinary or charge named in a blazon (grammar v2)."""

    kind: str
    count: int
    paint: Paint
    text: str  # words that follow the keyword, up to the next piece
    arrangement: str = ""  # "fasce" (in a row), "chef" (in the upper flank)
    on: Piece | None = None  # "chargé de": drawn on this piece
    around: Piece | None = None  # "accompagné de" / "côtoyé de": around this piece


def _solid(color: Color) -> Image.Image:
    return Image.new("RGB", (CANVAS, CANVAS), color)


def _two_tone(first: Color, second: Color, pick) -> Image.Image:
    """Pattern image: ``second`` where ``pick(x, y)`` (pixel indices) is true."""
    mask = Image.new("L", (CANVAS, CANVAS), 0)
    pixels = mask.load()
    for y in range(CANVAS):
        for x in range(CANVAS):
            if pick(x, y):
                pixels[x, y] = 255
    image = _solid(first)
    image.paste(_solid(second), mask=mask)
    return image


def ermine_pattern(spacing: float = 0.14) -> Image.Image:
    """Argent sown with sable ermine spots (``spacing`` in canvas fractions)."""
    image = _solid(ERMINE_FIELD)
    draw = ImageDraw.Draw(image)
    color = TINCTURES["sable"]
    spot = [(0, -0.5), (0.25, 0.3), (0.5, 0.5), (0, 0.35), (-0.5, 0.5), (-0.25, 0.3)]
    rows = int(1.0 / spacing) + 2
    for row in range(rows):
        for col in range(rows):
            cx = 0.06 + col * spacing + (spacing / 2 if row % 2 else 0.0)
            cy = 0.08 + row * spacing * 0.9
            draw.polygon(_transform(spot, cx, cy, spacing * 0.5), fill=color)
            radius = spacing * 0.065 * CANVAS
            for dx in (-0.13, 0.0, 0.13):
                x = (cx + dx * spacing) * CANVAS
                y = (cy - spacing * 0.36) * CANVAS
                draw.ellipse(
                    (x - radius, y - radius, x + radius, y + radius), fill=color
                )
    return image


def vair_pattern(rows: int = 7) -> Image.Image:
    """Vair: rows of argent bells on azure, alternate rows offset by half a bell."""
    image = _solid(TINCTURES["azur"])
    draw = ImageDraw.Draw(image)
    height = CANVAS / rows
    width = height
    for row in range(rows + 1):
        top = row * height
        x = -width + (width / 2 if row % 2 else 0.0)
        while x < CANVAS + width:
            bell = [
                (x + width * 0.5, top),
                (x + width * 0.78, top + height * 0.35),
                (x + width * 0.78, top + height * 0.7),
                (x + width, top + height),
                (x, top + height),
                (x + width * 0.22, top + height * 0.7),
                (x + width * 0.22, top + height * 0.35),
            ]
            draw.polygon(bell, fill=TINCTURES["argent"])
            x += width
    return image


def checky_pattern(
    first: Color, second: Color, cell: float, origin_y: float = 0.0
) -> Image.Image:
    """Checky of ``cell`` (canvas fraction), rows aligned on ``origin_y``."""
    size = cell * CANVAS
    oy = origin_y * CANVAS
    return _two_tone(
        first,
        second,
        lambda x, y: (int(x // size) + int((y - oy) // size)) % 2 == 1,
    )


def compony_pattern(first: Color, second: Color, cell: float = 0.12) -> Image.Image:
    """Compony along a bend (dexter chief to sinister base): cuts across the bend."""
    size = cell * CANVAS * 1.41
    return _two_tone(first, second, lambda x, y: int((x + y) // size) % 2 == 1)


def lozengy_bendwise(first: Color, second: Color, pieces: int = 5) -> Image.Image:
    """Fuselé en bande: tall lozenges leaning along the bend."""
    angle = math.radians(28)
    cos_a, sin_a = math.cos(angle), math.sin(angle)
    half_w = CANVAS / pieces / 2
    half_h = half_w * 1.9

    def pick(x: int, y: int) -> bool:
        u = x * cos_a - y * sin_a
        v = x * sin_a + y * cos_a
        a = math.floor(u / half_w + v / half_h)
        b = math.floor(u / half_w - v / half_h)
        return (a + b) % 2 == 0

    return _two_tone(first, second, pick)


def barry_pattern(first: Color, second: Color, pieces: int) -> Image.Image:
    """Horizontal stripes (``burelé``, ``fascé``), ``first`` at the top."""
    band = CANVAS / pieces
    return _two_tone(first, second, lambda x, y: int(y // band) % 2 == 1)


def bendy_pattern(first: Color, second: Color, pieces: int) -> Image.Image:
    """Bendy (``bandé``), ``first`` in the dexter chief."""
    band = CANVAS * 2 / pieces
    return _two_tone(first, second, lambda x, y: int((x - y + CANVAS) // band) % 2 == 1)


def paly_pattern(first: Color, second: Color, pieces: int) -> Image.Image:
    """Vertical stripes (``palé``)."""
    band = CANVAS / pieces
    return _two_tone(first, second, lambda x, y: int(x // band) % 2 == 1)


def tincture_paint(name: str) -> Paint:
    """Colour of a tincture, or pattern image for the furs."""
    if name == "hermine":
        return ermine_pattern()
    if name == "vair":
        return vair_pattern()
    return TINCTURES[name]


def _fill(image: Image.Image, mask: Image.Image, paint: Paint) -> None:
    source = paint if isinstance(paint, Image.Image) else _solid(paint)
    image.paste(source, mask=mask)


def _new_mask() -> tuple[Image.Image, ImageDraw.ImageDraw]:
    mask = Image.new("L", (CANVAS, CANVAS), 0)
    return mask, ImageDraw.Draw(mask)


def _full_mask() -> Image.Image:
    return Image.new("L", (CANVAS, CANVAS), 255)


def _shield_mask() -> Image.Image:
    mask, draw = _new_mask()
    draw.polygon(shield_polygon(), fill=255)
    return mask


def _inset(mask: Image.Image, fraction: float) -> Image.Image:
    return mask.filter(ImageFilter.MinFilter(int(fraction * CANVAS) | 1))


def _combine(masks) -> Image.Image:
    result, _ = _new_mask()
    for mask in masks:
        result = ImageChops.lighter(result, mask)
    return result


def _line_mask(points, width: float) -> Image.Image:
    mask, draw = _new_mask()
    draw.line(_px(points), fill=255, width=int(width * CANVAS), joint="curve")
    return mask


def _disc(draw: ImageDraw.ImageDraw, x: float, y: float, radius: float) -> None:
    """Filled disc at normalized (x, y), radius in canvas fractions."""
    cx, cy, r = x * CANVAS, y * CANVAS, radius * CANVAS
    draw.ellipse((cx - r, cy - r, cx + r, cy + r), fill=255)


# --- Field -------------------------------------------------------------------------

_FIELD_DIVIDED = re.compile(
    r"^(bande|burele|fasce|pale|echiquete|fusele en bande|fusele|losange)\s+"
    rf"{_OF}({_TINCTURE_WORDS})\s+et\s+{_OF}({_TINCTURE_WORDS})"
    r"(?:\s+de\s+(\w+)\s+pieces)?"
)
_FIELD_SALTIRE = re.compile(
    rf"^ecartele en sautoir\s+{_OF}({_TINCTURE_WORDS})\s+et\s+{_OF}({_TINCTURE_WORDS})"
)
_FIELD_PER_FESS = re.compile(
    rf"^coupe\s+{_OF}({_TINCTURE_WORDS})\s+et\s+{_OF}({_TINCTURE_WORDS})"
)
_FIELD_PLAIN = re.compile(rf"^{_OF}({_TINCTURE_WORDS})\b(?:\s+plain)?")
_SEME = re.compile(
    rf"^[\s,]*seme de (fleurs de lis|billettes)\s+{_OF}({_TINCTURE_WORDS})"
)


def _divided_paint(kind: str, first: Color, second: Color, count: int) -> Paint:
    if kind == "bande":
        return bendy_pattern(first, second, count or 6)
    if kind in ("burele", "fasce"):
        return barry_pattern(first, second, count or (10 if kind == "burele" else 6))
    if kind == "pale":
        return paly_pattern(first, second, count or 6)
    if kind == "echiquete":
        return checky_pattern(first, second, 1 / (count or 6))
    return lozengy_bendwise(first, second)


def _draw_field_v2(image: Image.Image, text: str) -> str:
    """Paint the field described at the start of ``text``; return the rest."""
    if match := _FIELD_DIVIDED.match(text):
        kind, first, second, pieces = match.groups()
        paint = _divided_paint(
            kind, TINCTURES[first], TINCTURES[second], NUMBERS.get(pieces or "", 0)
        )
        _fill(image, _full_mask(), paint)
    elif match := _FIELD_SALTIRE.match(text):
        first, second = TINCTURES[match[1]], TINCTURES[match[2]]
        image.paste(_solid(second))
        draw = ImageDraw.Draw(image)
        draw.polygon(_px([(0, 0), (1, 0), (0.5, 0.5)]), fill=first)
        draw.polygon(_px([(0, 1), (1, 1), (0.5, 0.5)]), fill=first)
    elif match := _FIELD_PER_FESS.match(text):
        image.paste(_solid(TINCTURES[match[2]]))
        ImageDraw.Draw(image).rectangle(
            _px([(0, 0), (1, 0.47)]), fill=TINCTURES[match[1]]
        )
    elif match := _FIELD_PLAIN.match(text):
        _fill(image, _full_mask(), tincture_paint(match[1]))
    else:
        raise ValueError(f"unreadable field: {text!r}")
    rest = text[match.end() :]
    seme = _SEME.match(rest)
    if seme:
        color = TINCTURES[seme[2]]
        draw = ImageDraw.Draw(image)
        if seme[1] == "billettes":
            for row in range(8):
                for col in range(7):
                    cx = 0.1 + col * 0.14 + (0.07 if row % 2 else 0.0)
                    cy = 0.08 + row * 0.12
                    draw.rectangle(
                        _px([(cx - 0.02, cy - 0.035), (cx + 0.02, cy + 0.035)]),
                        fill=color,
                    )
        else:
            _draw_semé(draw, color)
        rest = rest[seme.end() :]
    return rest


# --- Pieces ------------------------------------------------------------------------


def parse_pieces(text: str) -> list[Piece]:
    """Ordinaries and charges named in ``text`` (the blazon after its field)."""
    matches = list(_PIECE_RE.finditer(text))
    pieces: list[Piece] = []
    pending = ""
    previous_end = 0
    for index, match in enumerate(matches):
        keyword = match.group(1)
        before = text[: match.start()].split()
        between = text[previous_end : match.start()]
        stop = matches[index + 1].start() if index + 1 < len(matches) else len(text)
        after = text[match.end() : stop]
        previous_end = match.end()
        if before and before[-1] == "en":
            # "posées en fasce" arranges the previous charges, "accompagnée en chef
            # d'une étoile" places the next one, "passée en sautoir" is an attitude.
            if keyword == "fasce" and pieces:
                pieces[-1].arrangement = "fasce"
            elif keyword == "chef" and re.match(r"\s*d'une?\b", after):
                pending = "chef"
            continue
        kind = PIECES[keyword]
        count = NUMBERS.get(before[-1], 0) if before else 0
        if not count:
            count = PLURAL_DEFAULT if keyword.endswith(("s", "x")) else 1
        pair = _PAIR_RE.search(after)
        tincture = _TINCTURE_RE.search(after)
        if pair and (not tincture or pair.start() <= tincture.start()):
            first, second = TINCTURES[pair[1]], TINCTURES[pair[2]]
            if kind == "fasce":
                paint: Paint = checky_pattern(first, second, 0.22 / 3, origin_y=0.34)
            else:
                paint = compony_pattern(first, second)
        elif tincture:
            paint = tincture_paint(tincture[1])
        else:
            paint = pieces[-1].paint if pieces else TINCTURES["or"]
        piece = Piece(kind, count, paint, after, arrangement=pending)
        pending = ""
        if pieces and "charge" in between:
            piece.on = pieces[-1]
        elif pieces and ("accompagne" in between or "cotoye" in between):
            piece.around = pieces[-1]
        pieces.append(piece)
    return pieces


def _second_tincture(piece: Piece, word: str, default: str) -> Color:
    match = re.search(rf"{word}\w*\s+{_OF}({_TINCTURE_WORDS})\b", piece.text)
    name = match[1] if match and match[1] in TINCTURES else default
    return TINCTURES[name]


def _layout(piece: Piece, region: Region) -> tuple[list[tuple[float, float]], float]:
    """Centres and scale of the ``piece.count`` charges inside ``region``."""
    x0, y0, x1, y1 = region
    w, h = x1 - x0, y1 - y0
    small = min(w, h)

    def at(fx: float, fy: float) -> tuple[float, float]:
        return (x0 + fx * w, y0 + fy * h)

    n = piece.count
    if piece.around is not None and piece.around.kind in ("bande", "baton"):
        if n == 1 or piece.arrangement == "chef":
            return [(0.74, 0.24)], 0.22
        upper = [(0.6, 0.15), (0.82, 0.15), (0.84, 0.37)][: (n + 1) // 2]
        lower = [(0.16, 0.5), (0.18, 0.72), (0.4, 0.8)][: n // 2]
        return upper + lower, 0.17
    if piece.arrangement == "chef":
        return [(0.74, 0.22)], 0.22
    if piece.arrangement == "fasce":
        step = w / n
        return [(x0 + step * (i + 0.5), y0 + 0.42 * h) for i in range(n)], step * 1.3
    if n == 1:
        return [at(0.5, 0.46)], 0.66 * small
    if n == 2:
        return [at(0.5, 0.27), at(0.5, 0.68)], 0.36 * small
    if n == 3:
        return [at(0.27, 0.24), at(0.73, 0.24), at(0.5, 0.66)], 0.36 * small
    rows = [3, 2, 1] if n == 6 else [n]
    spots = [
        at(0.5 + (c - (count - 1) / 2) * 0.3, 0.2 + r * 0.28)
        for r, count in enumerate(rows)
        for c in range(count)
    ]
    return spots, 0.24 * small


PERRON_BOXES = [
    (-0.36, 0.30, 0.36, 0.42),
    (-0.26, 0.20, 0.26, 0.30),
    (-0.16, 0.10, 0.16, 0.20),
    (-0.05, -0.30, 0.05, 0.10),
    (-0.12, -0.52, 0.12, -0.47),
    (-0.025, -0.62, 0.025, -0.40),
]


def _charge_mask(piece: Piece, spots, scale: float) -> Image.Image:
    mask, draw = _new_mask()
    if piece.kind == "leopard" and piece.count == 3 and not piece.arrangement:
        for index in range(3):
            shape = _transform(LEOPARD, 0.5, 0.2 + index * 0.24, 0.5 - index * 0.06)
            draw.polygon(shape, fill=255)
        return mask
    for cx, cy in spots:
        if piece.kind == "tourteau":
            _disc(draw, cx, cy, scale * 0.42)
        elif piece.kind == "rose":
            for k in range(5):
                angle = k * math.tau / 5 - math.pi / 2
                px = cx + math.cos(angle) * scale * 0.21
                py = cy + math.sin(angle) * scale * 0.21
                _disc(draw, px, py, scale * 0.21)
        elif piece.kind == "perron":
            for bx0, by0, bx1, by1 in PERRON_BOXES:
                box = [(cx + bx0 * scale, cy + by0 * scale)]
                box.append((cx + bx1 * scale, cy + by1 * scale))
                draw.rectangle(_px(box), fill=255)
            _disc(draw, cx, cy - 0.36 * scale, 0.08 * scale)
        else:
            if piece.kind == "dauphin":
                _draw_dolphin(draw, cx, cy, scale)
                continue
            if piece.kind == "nef":
                _draw_ship_shape(draw, cx, cy, scale)
                continue
            draw.polygon(_transform(SHAPES[piece.kind], cx, cy, scale), fill=255)
            if piece.kind == "coussin":
                for dx, dy in ((-0.4, -0.4), (0.4, -0.4), (0.4, 0.4), (-0.4, 0.4)):
                    _disc(draw, cx + dx * scale, cy + dy * scale, 0.07 * scale)
    return mask


def _draw_dolphin(
    draw: ImageDraw.ImageDraw, cx: float, cy: float, scale: float
) -> None:
    """Dolphin body: thick curved line tapering to the tail, round head, snout."""
    spine = _transform(DOLPHIN_SPINE, cx, cy, scale)
    for start in range(len(spine) - 1):
        width = int((0.2 - 0.022 * start) * scale * CANVAS)
        draw.line(spine[start : start + 2], fill=255, width=width, joint="curve")
        x, y = spine[start + 1]
        radius = width / 2
        draw.ellipse((x - radius, y - radius, x + radius, y + radius), fill=255)
    _disc(draw, cx - 0.24 * scale, cy - 0.28 * scale, 0.13 * scale)
    snout = [(-0.34, -0.34), (-0.52, -0.24), (-0.34, -0.2)]
    draw.polygon(_transform(snout, cx, cy, scale), fill=255)
    draw.polygon(_transform(DOLPHIN_TAIL, cx, cy, scale), fill=255)


def _dolphin_fins(
    draw: ImageDraw.ImageDraw, cx: float, cy: float, scale: float
) -> None:
    for fin in DOLPHIN_FINS:
        draw.polygon(_transform(fin, cx, cy, scale), fill=255)


def _wavy_barre(offset: float, width: float) -> Image.Image:
    """Wavy sinister bend (``barre ondée``) shifted by ``offset`` across the field."""
    points = []
    for i in range(41):
        t = -0.1 + 1.2 * i / 40
        wave = 0.025 * math.sin(t * math.tau * 3)
        points.append((1.0 - t + offset + wave, t + offset + wave))
    return _line_mask(points, width)


def _ordinary_mask(piece: Piece) -> Image.Image:
    kind, n = piece.kind, piece.count
    mask, draw = _new_mask()
    if kind == "chef":
        draw.rectangle(_px([(0, 0), (1, 0.28)]), fill=255)
    elif kind == "franc_quartier":
        draw.rectangle(_px([(0, 0), (0.42, 0.4)]), fill=255)
    elif kind == "bordure":
        shield = _shield_mask()
        return ImageChops.subtract(shield, _inset(shield, 0.09))
    elif kind == "orle":
        shield = _shield_mask()
        return ImageChops.subtract(_inset(shield, 0.1), _inset(shield, 0.19))
    elif kind == "lambel":
        _draw_label(draw, 255)
    elif kind == "fasce" and n == 1:
        draw.rectangle(_px([(0, 0.34), (1, 0.56)]), fill=255)
    elif kind == "fasce":
        for i in range(n):
            y = 0.2 + (i + 0.5) * 0.62 / n
            draw.rectangle(_px([(0, y - 0.065), (1, y + 0.065)]), fill=255)
    elif kind == "pal":
        _draw_pals(draw, 255, n)
    elif kind == "croix":
        _draw_cross(draw, 255)
    elif kind == "sautoir":
        return _combine(
            [
                _line_mask([(-0.05, -0.05), (1.05, 1.05)], 0.17),
                _line_mask([(1.05, -0.05), (-0.05, 1.05)], 0.17),
            ]
        )
    elif kind == "bande":
        return _line_mask([(-0.1, -0.1), (1.1, 1.1)], 0.2)
    elif kind == "baton":
        return _line_mask([(-0.1, -0.1), (1.1, 1.1)], 0.11)
    elif kind == "barre":
        offsets = [(i - (n - 1) / 2) * 0.3 for i in range(n)]
        if "onde" in piece.text:
            return _combine(_wavy_barre(offset, 0.1) for offset in offsets)
        return _combine(
            _line_mask([(1.1 + o, -0.1 + o), (-0.1 + o, 1.1 + o)], 0.1) for o in offsets
        )
    elif kind == "cotice":
        return _combine(
            _line_mask([(-0.1 + o, -0.1 - o), (1.1 + o, 1.1 - o)], 0.04)
            for o in (-0.12, 0.12)
        )
    elif kind == "chevron":
        width = int((0.1 if n > 1 else 0.16) * CANVAS)
        for i in range(n):
            apex = 0.14 + i * 0.22 if n > 1 else 0.3
            legs = [(0.0, apex + 0.46), (0.5, apex), (1.0, apex + 0.46)]
            draw.line(_px(legs), fill=255, width=width, joint="curve")
    elif kind == "trescheur":
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
            draw.line(_px(outline), fill=255, width=width, joint="curve")
        for fx, fy in ((0.5, 0.16), (0.16, 0.34), (0.84, 0.34), (0.3, 0.7), (0.7, 0.7)):
            draw.polygon(_transform(FLEUR_DE_LIS, fx, fy, 0.07), fill=255)
    return mask


def _draw_on_label(image: Image.Image, piece: Piece) -> None:
    """Small charges on the three pendants of a label (Artois castles, Lancastre lis)."""
    mask, draw = _new_mask()
    shape = SHAPES.get(piece.kind, CASTLE)
    for cx in (0.26, 0.5, 0.74):
        draw.polygon(_transform(shape, cx, 0.225, 0.075), fill=255)
    _fill(image, mask, piece.paint)


def _draw_on_chief(image: Image.Image, piece: Piece) -> None:
    mask, draw = _new_mask()
    if piece.kind == "croix":
        draw.rectangle(_px([(0.45, 0.0), (0.55, 0.28)]), fill=255)
        draw.rectangle(_px([(0.0, 0.1), (1.0, 0.18)]), fill=255)
    else:
        for cx in (0.3, 0.5, 0.7)[: piece.count]:
            shape = SHAPES.get(piece.kind, STAR)
            draw.polygon(_transform(shape, cx, 0.14, 0.16), fill=255)
    _fill(image, mask, piece.paint)


# Charges drawn from the vendored SVG (lot DA1b) instead of polygons.
VECTOR_KINDS = {"lion", "lionceau", "leopard", "aigle", "chateau", "dauphin"}


def _draw_vector_charge(
    image: Image.Image, piece: Piece, spots: list[tuple[float, float]], scale: float
) -> None:
    if piece.kind in ("lion", "lionceau"):
        charge, crowned = lion_variant(piece.text)
    elif piece.kind == "leopard":
        lionne = "lionne" in piece.text.split()[:2]
        charge, crowned = ("leopard_lionne" if lionne else "leopard"), False
    else:
        charge, crowned = piece.kind, False
    paints = charge_paints(charge, piece.paint, piece.text)
    if piece.kind == "dauphin":
        paints["armed"] = _second_tincture(piece, "peautre", "gueules")
    if charge == "leopard" and piece.count == 3 and not piece.arrangement:
        for index in range(3):
            width = 0.7 - index * 0.06
            spot = [(0.5, 0.2 + index * 0.25)]
            paint_charge(image, charge, spot, width, paints, height=0.24)
        return
    if charge == "leopard":
        paint_charge(image, charge, spots, scale, paints, height=scale * 0.62)
        return
    paint_charge(image, charge, spots, scale, paints, crowned=crowned)


def _draw_charge(image: Image.Image, piece: Piece, region: Region) -> None:
    spots, scale = _layout(piece, region)
    if piece.kind in VECTOR_KINDS:
        if piece.kind in ("leopard", "aigle", "dauphin") and piece.count == 1:
            scale *= 1.2
        _draw_vector_charge(image, piece, spots, scale)
        return
    if piece.kind == "leopard" and piece.count == 1:
        scale *= 1.25
    if piece.kind in ("gonfanon", "dauphin") and piece.count == 1:
        scale *= 1.2
    _fill(image, _charge_mask(piece, spots, scale), piece.paint)
    if piece.kind == "rose":
        centre, draw = _new_mask()
        for cx, cy in spots:
            _disc(draw, cx, cy, scale * 0.13)
        _fill(image, centre, _second_tincture(piece, "boutonne", "or"))
    elif piece.kind == "dauphin":
        fins, draw = _new_mask()
        for cx, cy in spots:
            _dolphin_fins(draw, cx, cy, scale)
        _fill(image, fins, _second_tincture(piece, "peautre", "gueules"))
    elif piece.kind == "gonfanon":
        fringe, draw = _new_mask()
        for cx, cy in spots:
            tips = _transform(GONFANON[2:], cx, cy, scale)
            draw.line(tips, fill=255, width=int(0.016 * CANVAS), joint="curve")
            for fx in (-0.3, 0.0, 0.3):
                ring = [(fx - 0.05, -0.46), (fx + 0.05, -0.36)]
                draw.ellipse(_transform(ring, cx, cy, scale), outline=255, width=6)
        _fill(image, fringe, _second_tincture(piece, "frange", "sinople"))


def _draw_pieces(image: Image.Image, pieces: list[Piece]) -> None:
    has_chief = any(p.kind == "chef" for p in pieces)
    inner = any(p.kind in ("bordure", "orle", "trescheur") for p in pieces)
    region = (
        0.14 if inner else 0.08,
        0.3 if has_chief else (0.12 if inner else 0.06),
        0.86 if inner else 0.92,
        0.84 if inner else 0.88,
    )
    for piece in pieces:
        if piece.on is not None and piece.on.kind == "lambel":
            _draw_on_label(image, piece)
        elif piece.on is not None and piece.on.kind == "chef":
            _draw_on_chief(image, piece)
        elif piece.kind in ORDINARIES:
            _fill(image, _ordinary_mask(piece), piece.paint)
        else:
            _draw_charge(image, piece, region)


_QUARTERLY = re.compile(
    r"^ecartele\s*:\s*aux 1 et 4\s+(.+?)\s*[;,]\s*aux 2 et 3\s+(.+)$"
)


def is_quarterly(blazon: str) -> bool:
    """Whether ``blazon`` reads ``écartelé : aux 1 et 4 … ; aux 2 et 3 …``."""
    return _QUARTERLY.match(normalize_blazon(blazon)) is not None


def normalize_blazon(blazon: str) -> str:
    """Accent-free lower-case blazon without parentheses or final full stop."""
    text = _normalize(blazon).replace("’", "'")
    return re.sub(r"\(.*?\)", "", text).strip().rstrip(".").strip()


def render_house_field(blazon: str) -> Image.Image:
    """Paint ``blazon`` (grammar v2) on a CANVAS × CANVAS RGB square."""
    text = normalize_blazon(blazon)
    image = Image.new("RGB", (CANVAS, CANVAS))
    quarterly = _QUARTERLY.match(text)
    if quarterly:
        half = CANVAS // 2
        size = (half, half)
        first = render_house_field(quarterly[1]).resize(size, Image.Resampling.LANCZOS)
        second = render_house_field(quarterly[2]).resize(size, Image.Resampling.LANCZOS)
        image.paste(first, (0, 0))
        image.paste(second, (half, 0))
        image.paste(second, (0, half))
        image.paste(first, (half, half))
        return image
    rest = _draw_field_v2(image, text)
    _draw_pieces(image, parse_pieces(rest))
    return image


def load_houses(path: Path = HOUSES_FILE) -> dict:
    """The house arms file (``houses`` list and faction ``badges``)."""
    return json.loads(path.read_text(encoding="utf-8"))


def shield_for_house(house: dict, factions: dict[str, dict]) -> Image.Image:
    """Shield of one house: the faction drawing for ``arms_of``, grammar v2 otherwise."""
    if house.get("arms_of"):
        return shield_for_faction(factions[house["arms_of"]])
    return finish_shield(render_house_field(house["blazon"]))


def build_houses(
    houses_file: Path = HOUSES_FILE, out_dir: Path = HOUSES_DIR
) -> list[Path]:
    """Write ``<out_dir>/<house id>.png`` for every house and return the paths."""
    out_dir.mkdir(parents=True, exist_ok=True)
    factions = {faction["id"]: faction for faction in load_factions()}
    houses = sorted(load_houses(houses_file)["houses"], key=lambda h: h["id"])
    written = []
    for house in houses:
        path = out_dir / f"{house['id']}.png"
        shield_for_house(house, factions).save(path, optimize=True)
        written.append(path)
    return written
