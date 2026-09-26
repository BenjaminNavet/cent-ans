"""Heraldic banners and pennons for the 3D map markers and battle regiments (F10c).

Reuses the blazon interpretation of :mod:`cent_ans_tools.heraldry` but paints the arms on
the whole cloth instead of a shield. Flat saturated colours, no baked lighting: the Godot
shader adds the cloth shading. Output (RGBA, powers of two, transparent outside the cloth):

- ``fac_<id>_banner.png`` 256×512: vertical banner of arms (banneret), cloth on top,
  slightly irregular lower edge, no pole.
- ``fac_<id>_pennon.png`` 512×128: pennon, arms at the hoist (left), tapering swallow tail
  in the livery colours.
- ``fac_<id>_standard.png`` 1024×256 (lot EP5): long tapering standard of a great lord,
  arms at the hoist, livery stripes sown with badges, split rounded tail.
- ``oriflamme.png``, ``st_george.png``, ``dragon.png`` 256×512: royal battle standards
  (oriflamme of Saint-Denis, cross of St George, dragon banner raised at Crécy in 1346).
"""

from __future__ import annotations

import math
import re
from dataclasses import replace
from pathlib import Path

from PIL import Image, ImageDraw

from cent_ans_tools import heraldry
from cent_ans_tools.heraldry import CANVAS, TINCTURES, Blazon, Color

BANNERS_DIR = heraldry.HERALDRY_DIR / "banners"

BANNER_SIZE = (256, 512)
BANNER_CLOTH_HEIGHT = 352  # cloth of arms, ratio ≈ 1:1.4 (charges barely stretched)
PENNON_SIZE = (512, 128)
STANDARD_SIZE = (1024, 256)
SUPERSAMPLE = 2

# Dragon passant facing the hoist (normalised square canvas), after the English dragon
# standard of Crécy: body with legs and curled tail, and one raised bat wing.
DRAGON_BODY = [
    (0.10, 0.34), (0.16, 0.27), (0.26, 0.26), (0.30, 0.31), (0.25, 0.35),
    (0.33, 0.44), (0.48, 0.47), (0.66, 0.47), (0.80, 0.52), (0.92, 0.50),
    (0.97, 0.60), (0.90, 0.70), (0.84, 0.66), (0.90, 0.61), (0.86, 0.57),
    (0.76, 0.60), (0.75, 0.76), (0.69, 0.76), (0.68, 0.63), (0.52, 0.63),
    (0.50, 0.78), (0.44, 0.78), (0.45, 0.61), (0.34, 0.55), (0.24, 0.42),
    (0.18, 0.38),
]  # fmt: skip
DRAGON_WING = [
    (0.42, 0.47), (0.44, 0.10), (0.52, 0.16), (0.62, 0.12), (0.64, 0.24),
    (0.74, 0.22), (0.72, 0.34), (0.80, 0.36), (0.66, 0.48),
]  # fmt: skip


def _full_cloth_polygon() -> list[tuple[float, float]]:
    """Whole square canvas: bordures follow the cloth edge, not a shield outline.

    Inset by one pixel so that the erosion used by :func:`heraldry._draw_border` sees
    an edge (a mask filling the whole image would never erode).
    """
    return [(1, 1), (CANVAS - 2, 1), (CANVAS - 2, CANVAS - 2), (1, CANVAS - 2)]


def _draw_square_tressure(field: Image.Image, color: Color) -> None:
    """Double tressure following the edge of the cloth (Scotland), not a shield."""
    draw = ImageDraw.Draw(field)
    width = int(0.018 * CANVAS)
    for inset in (0.07, 0.11):
        low, high = inset * CANVAS, (1 - inset) * CANVAS
        draw.rectangle((low, low, high, high), outline=color, width=width)


def render_arms(blazon: Blazon) -> Image.Image:
    """Arms painted on a full square (CANVAS × CANVAS, RGB), flat colours."""
    if heraldry.is_quarterly(blazon.text):
        # Castile, Hainaut: drawn with the grammar v2 like their shields (DA1b).
        return render_house_arms(blazon.text)
    tressure = blazon.has("trescheur")
    if tressure:
        blazon = replace(blazon, text=blazon.text.replace("trescheur", "--"))
    original = heraldry.shield_polygon
    heraldry.shield_polygon = _full_cloth_polygon
    try:
        field = Image.new("RGB", (CANVAS, CANVAS))
        heraldry._draw_field(ImageDraw.Draw(field), blazon)
        heraldry._draw_charges(field, blazon)
    finally:
        heraldry.shield_polygon = original
    if tressure:
        _draw_square_tressure(field, blazon.charge)
    return field


def render_house_arms(blazon: str) -> Image.Image:
    """House arms (grammar v2) on a full square, bordures along the cloth edge."""
    original = heraldry.shield_polygon
    heraldry.shield_polygon = _full_cloth_polygon
    try:
        return heraldry.render_house_field(blazon)
    finally:
        heraldry.shield_polygon = original


def house_livery(blazon: str) -> tuple[Color, Color]:
    """Livery of a house standard: the first two distinct tinctures of its blazon."""
    names = re.findall(
        r"\b(or|argent|gueules|azur|sable|sinople|pourpre)\b",
        heraldry.normalize_blazon(blazon),
    )
    colors = list(dict.fromkeys(TINCTURES[name] for name in names))
    if not colors:
        return (TINCTURES["argent"], TINCTURES["gueules"])
    if len(colors) == 1:
        colors.append(
            TINCTURES["argent"]
            if colors[0] != TINCTURES["argent"]
            else TINCTURES["gueules"]
        )
    return (colors[0], colors[1])


def _blazon_of(faction: dict) -> Blazon:
    arms = faction["heraldry"]
    return heraldry.parse_blazon(
        arms.get("blazon", ""), arms["primary_color"], arms["secondary_color"]
    )


def _wavy_edge_mask(
    width: int, cloth_height: int, height: int, seed: int
) -> Image.Image:
    """Alpha mask of a cloth whose lower edge wavers by a few pixels."""
    mask = Image.new("L", (width, height), 0)
    amplitude = cloth_height * 0.012
    phase = (seed % 97) / 97 * math.tau
    edge = [
        (
            x,
            cloth_height
            - amplitude
            + amplitude * math.sin(phase + x / width * math.tau * 1.5),
        )
        for x in range(0, width + 1, max(1, width // 64))
    ]
    ImageDraw.Draw(mask).polygon([(0, 0), (width, 0), *edge[::-1]], fill=255)
    return mask


def _finish(image: Image.Image, size: tuple[int, int]) -> Image.Image:
    return image.resize(size, Image.Resampling.LANCZOS)


def render_banner(arms: Image.Image, seed: int = 0) -> Image.Image:
    """Vertical banner of arms, 256×512 RGBA, cloth at the top."""
    width, height = (value * SUPERSAMPLE for value in BANNER_SIZE)
    cloth_height = BANNER_CLOTH_HEIGHT * SUPERSAMPLE
    cloth = arms.resize((width, cloth_height), Image.Resampling.LANCZOS)
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    canvas.paste(
        cloth, (0, 0), _wavy_edge_mask(width, cloth_height, cloth_height, seed)
    )
    return _finish(canvas, BANNER_SIZE)


def render_pennon(arms: Image.Image, livery: tuple[Color, Color]) -> Image.Image:
    """Pennon 512×128 RGBA: arms at the hoist, swallow tail in two livery stripes."""
    width, height = (value * SUPERSAMPLE for value in PENNON_SIZE)
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    hoist = height  # square of arms
    tip, notch = width, width * 0.86
    upper = [
        (hoist, 0),
        (tip, height * 0.32),
        (notch, height * 0.5),
        (hoist, height * 0.5),
    ]
    lower = [
        (hoist, height * 0.5),
        (notch, height * 0.5),
        (tip, height * 0.68),
        (hoist, height),
    ]
    draw.polygon(upper, fill=(*livery[0], 255))
    draw.polygon(lower, fill=(*livery[1], 255))
    canvas.paste(arms.resize((hoist, hoist), Image.Resampling.LANCZOS), (0, 0))
    return _finish(canvas, PENNON_SIZE)


def render_standard(arms: Image.Image, livery: tuple[Color, Color]) -> Image.Image:
    """Long standard 1024×256 RGBA (EP5): arms at the hoist, tapering livery fly.

    The standard of a great lord, late 14th century: a square of arms (or the cross of
    the realm) at the hoist, then the fly parted per fess in the livery colours, sown with
    roundels of the other tincture, narrowing to a split, rounded tail.
    """
    width, height = (value * SUPERSAMPLE for value in STANDARD_SIZE)
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    hoist = height

    def edge(x: float) -> float:
        """Half height of the fly at `x` (tapering from the hoist to the tail)."""
        t = (x - hoist) / (width - hoist)
        return height * 0.5 * (1.0 - 0.45 * t)

    xs = [hoist + (width - hoist) * i / 32 for i in range(33)]
    mid = height * 0.5
    upper = [(hoist, 0.0)] + [(x, mid - edge(x)) for x in xs] + [(width, mid)]
    upper += [(width * 0.9, mid), (hoist, mid)]
    lower = [(hoist, mid), (width * 0.9, mid), (width, mid)]
    lower += [(x, mid + edge(x)) for x in reversed(xs)] + [(hoist, float(height))]
    draw.polygon(upper, fill=(*livery[0], 255))
    draw.polygon(lower, fill=(*livery[1], 255))
    # Split tail: a notch cut into the end of the fly.
    notch = [(width * 0.9, mid), (width + 1, mid - height * 0.12)]
    notch += [(width + 1, mid + height * 0.12)]
    draw.polygon(notch, fill=(0, 0, 0, 0))
    # Badges sown on the fly (roundels of the other tincture).
    radius = height * 0.07
    for index, x in enumerate(xs[2:-4:4]):
        for row, (y, color) in enumerate(((0.28, livery[1]), (0.72, livery[0]))):
            cy = mid + (y - 0.5) * 2 * edge(x) * 0.8
            offset = radius * 1.6 if (index + row) % 2 else 0.0
            box = (x + offset - radius, cy - radius, x + offset + radius, cy + radius)
            draw.ellipse(box, fill=(*color, 255))
    canvas.paste(arms.resize((hoist, hoist), Image.Resampling.LANCZOS), (0, 0))
    return _finish(canvas, STANDARD_SIZE)


def render_oriflamme() -> Image.Image:
    """Oriflamme of Saint-Denis: plain red silk ending in pointed tails."""
    width, height = (value * SUPERSAMPLE for value in BANNER_SIZE)
    canvas = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(canvas)
    body = height * 0.52
    tails = 3
    points: list[tuple[float, float]] = [(0, 0), (width, 0), (width, body)]
    for index in range(tails, 0, -1):
        right = width * index / tails
        left = width * (index - 1) / tails
        points += [((left + right) / 2, height * 0.94), (left, body)]
    draw.polygon(points, fill=(*TINCTURES["gueules"], 255))
    # Gold flames sown on the silk, as later depictions show.
    for row in range(3):
        for col in range(3):
            cx = width * (0.2 + col * 0.3 + (0.15 if row % 2 else 0.0))
            cy = body * (0.18 + row * 0.3)
            if cx > width * 0.9:
                continue
            flame = [
                (cx, cy - height * 0.05),
                (cx + width * 0.05, cy + height * 0.02),
                (cx, cy + height * 0.035),
                (cx - width * 0.05, cy + height * 0.02),
            ]
            draw.polygon(flame, fill=(*TINCTURES["or"], 255))
    return _finish(canvas, BANNER_SIZE)


def render_st_george() -> Image.Image:
    """Banner of St George: argent, a cross gules."""
    blazon = Blazon(
        field=TINCTURES["argent"], charge=TINCTURES["gueules"], text="croix"
    )
    return render_banner(render_arms(blazon), seed=23)


def render_dragon() -> Image.Image:
    """Dragon banner raised by Edward III at Crécy: a red dragon on white."""
    field = Image.new("RGB", (CANVAS, CANVAS), TINCTURES["argent"])
    draw = ImageDraw.Draw(field)
    for shape in (DRAGON_WING, DRAGON_BODY):
        draw.polygon(
            [(x * CANVAS, y * CANVAS) for x, y in shape], fill=TINCTURES["gueules"]
        )
    return render_banner(field, seed=46)


def build(
    factions_dir: Path = heraldry.FACTIONS_DIR, out_dir: Path = BANNERS_DIR
) -> list[Path]:
    """Write the banners, pennons and standards of every faction plus the specials."""
    out_dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for index, faction in enumerate(heraldry.load_factions(factions_dir)):
        blazon = _blazon_of(faction)
        arms = render_arms(blazon)
        livery = (blazon.field, blazon.charge)
        for kind, image in (
            ("banner", render_banner(arms, seed=index)),
            ("pennon", render_pennon(arms, livery)),
            ("standard", render_standard(arms, livery)),
        ):
            path = out_dir / f"{faction['id']}_{kind}.png"
            image.save(path, optimize=True)
            written.append(path)
    written += build_houses(out_dir / "houses")
    for name, image in (
        ("oriflamme", render_oriflamme()),
        ("st_george", render_st_george()),
        ("dragon", render_dragon()),
    ):
        path = out_dir / f"{name}.png"
        image.save(path, optimize=True)
        written.append(path)
    return written


def build_houses(out_dir: Path = BANNERS_DIR / "houses") -> list[Path]:
    """Banner, pennon and standard of every house (lot DA1b, standards of the general).

    Houses bearing the arms of a faction (``arms_of``) reuse its cloth drawing.
    """
    out_dir.mkdir(parents=True, exist_ok=True)
    factions = {faction["id"]: faction for faction in heraldry.load_factions()}
    houses = sorted(heraldry.load_houses()["houses"], key=lambda h: h["id"])
    written: list[Path] = []
    for index, house in enumerate(houses):
        if house.get("arms_of"):
            blazon = _blazon_of(factions[house["arms_of"]])
            arms = render_arms(blazon)
            livery = (blazon.field, blazon.charge)
        else:
            arms = render_house_arms(house["blazon"])
            livery = house_livery(house["blazon"])
        for kind, image in (
            ("banner", render_banner(arms, seed=100 + index)),
            ("pennon", render_pennon(arms, livery)),
            ("standard", render_standard(arms, livery)),
        ):
            path = out_dir / f"{house['id']}_{kind}.png"
            image.save(path, optimize=True)
            written.append(path)
    return written
