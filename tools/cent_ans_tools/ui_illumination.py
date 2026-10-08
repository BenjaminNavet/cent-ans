"""Illuminated-manuscript UI kit: 9-slice textures for the Godot parchment theme.

The interface borrows from 14th-century French books of hours (Jean Pucelle's workshop):
vellum pages, burnished gold leaf, lapis azure and vermilion "bar borders", ivy-leaf
rinceaux (vine scrolls) and white-lead filigree. Everything is drawn procedurally with
Pillow + numpy, so the kit is deterministic and free of third-party licences.

Every texture is a 9-slice (``StyleBoxTexture``): a square or strip whose central region is
exactly one period of a *periodic* vellum tile and whose border motifs repeat with the same
period, so Godot can tile edges and centre (``AXIS_STRETCH_MODE_TILE_FIT``) without seams.
The sidecar ``kit.json`` records the margins used by each texture.

Vector ornaments are drawn at :data:`SUPERSAMPLE` x resolution into grey-level masks, then
box-downsampled (anti-aliasing) and used to lay coloured "pigments" over the vellum.
"""

from __future__ import annotations

import json
import math
import shutil
from collections.abc import Callable
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.ndimage import gaussian_filter

REPO_DIR = Path(__file__).resolve().parents[2]
OUTPUT_DIR = REPO_DIR / "game" / "assets" / "ui" / "illumination"
SIDECAR_NAME = "kit.json"
SUPERSAMPLE = 4

# Pigments (linear-ish sRGB 0..1).
VELLUM = np.array([0.945, 0.890, 0.760])
VELLUM_WARM = np.array([0.860, 0.760, 0.560])
VELLUM_DARK = np.array([0.835, 0.745, 0.560])
INK = np.array([0.200, 0.120, 0.060])
GOLD_DARK = np.array([0.560, 0.390, 0.120])
GOLD_LIGHT = np.array([0.990, 0.870, 0.520])
AZURE = np.array([0.120, 0.215, 0.540])
AZURE_LIGHT = np.array([0.260, 0.400, 0.760])
GULES = np.array([0.700, 0.150, 0.090])
WHITE_LEAD = np.array([0.975, 0.955, 0.900])

Point = tuple[float, float]


# --------------------------------------------------------------------------------------
# Low-level helpers
# --------------------------------------------------------------------------------------


def periodic_noise(
    height: int, width: int, sigma: float, rng: np.random.Generator
) -> np.ndarray:
    """Return zero-mean, unit-std blurred noise that wraps around both axes."""
    field = gaussian_filter(rng.standard_normal((height, width)), sigma, mode="wrap")
    return (field - field.mean()) / (field.std() + 1e-9)


def vellum_tile(
    period: int,
    rng: np.random.Generator,
    base: np.ndarray = VELLUM,
    darkness: float = 1.0,
) -> np.ndarray:
    """Periodic vellum (``period`` x ``period`` x 3): blotches, hair follicles, grain."""
    blotch = periodic_noise(period, period, period / 7.0, rng)
    mottling = periodic_noise(period, period, 2.5, rng)
    grain = periodic_noise(period, period, 0.6, rng)
    warmth = np.clip(0.35 + 0.22 * blotch, 0.0, 1.0)[..., None]
    rgb = base * (1.0 - warmth * 0.25 * darkness) + VELLUM_WARM * (
        warmth * 0.25 * darkness
    )
    rgb = rgb * (1.0 + 0.018 * mottling[..., None] + 0.022 * grain[..., None])
    # Follicle speckles of calfskin: sparse, soft, slightly darker dots.
    speckles = np.zeros((period, period))
    count = period * period // 1400 + 2
    ys = rng.integers(0, period, count)
    xs = rng.integers(0, period, count)
    speckles[ys, xs] = 1.0
    speckles = gaussian_filter(speckles, 1.1, mode="wrap")
    rgb = rgb * (1.0 - 0.6 * np.clip(speckles, 0, 0.12)[..., None])
    return np.clip(rgb, 0.0, 1.0)


def tile_to(tile: np.ndarray, height: int, width: int, offset: int) -> np.ndarray:
    """Fill a ``height`` x ``width`` image so that ``image[offset:offset+T]`` equals ``tile``."""
    period_y, period_x = tile.shape[:2]
    rows = (np.arange(height) - offset) % period_y
    cols = (np.arange(width) - offset) % period_x
    return tile[rows][:, cols]


def metal_tile(
    period: int,
    rng: np.random.Generator,
    dark: np.ndarray,
    light: np.ndarray,
    sheen: float = 0.30,
) -> np.ndarray:
    """Burnished pigment tile: a broad diagonal sheen plus a little tooling noise."""
    yy, xx = np.mgrid[0:period, 0:period]
    wave = 0.5 + 0.5 * np.sin((xx + yy) * (2.0 * math.pi / period))
    tooling = periodic_noise(period, period, 1.6, rng)
    mix = np.clip(0.52 + sheen * (wave - 0.5) + 0.10 * tooling, 0.0, 1.0)[..., None]
    return dark * (1.0 - mix) + light * mix


def flat_tile(
    period: int, color: np.ndarray, rng: np.random.Generator, variation: float = 0.05
) -> np.ndarray:
    """Hand-laid pigment tile: a flat colour with a faint brush mottling."""
    mottle = periodic_noise(period, period, 1.8, rng)[..., None]
    return np.clip(color * (1.0 + variation * mottle), 0.0, 1.0)


class Mask:
    """A supersampled grey-level mask drawn with Pillow in *output* pixel units."""

    def __init__(self, width: int, height: int) -> None:
        """Create an empty mask of the given output size."""
        self.width = width
        self.height = height
        self.image = Image.new("L", (width * SUPERSAMPLE, height * SUPERSAMPLE), 0)
        self.draw = ImageDraw.Draw(self.image)

    @staticmethod
    def _s(value: float) -> float:
        return value * SUPERSAMPLE

    def rect(
        self, x0: float, y0: float, x1: float, y1: float, value: int = 255
    ) -> None:
        """Fill the rectangle [x0, x1) x [y0, y1)."""
        self.draw.rectangle(
            [self._s(x0), self._s(y0), self._s(x1) - 1, self._s(y1) - 1], fill=value
        )

    def frame(self, inset: float, width: float, value: int = 255) -> None:
        """Rectangular band of ``width`` pixels starting ``inset`` pixels from the edges."""
        w, h = self.width, self.height
        self.rect(inset, inset, w - inset, inset + width, value)
        self.rect(inset, h - inset - width, w - inset, h - inset, value)
        self.rect(inset, inset, inset + width, h - inset, value)
        self.rect(w - inset - width, inset, w - inset, h - inset, value)

    def polygon(self, points: list[Point], value: int = 255) -> None:
        """Fill a polygon."""
        self.draw.polygon([(self._s(x), self._s(y)) for x, y in points], fill=value)

    def disc(self, cx: float, cy: float, radius: float, value: int = 255) -> None:
        """Fill a disc."""
        r = self._s(radius)
        self.draw.ellipse(
            [self._s(cx) - r, self._s(cy) - r, self._s(cx) + r, self._s(cy) + r],
            fill=value,
        )

    def line(self, points: list[Point], width: float, value: int = 255) -> None:
        """Stroke a polyline with round joints."""
        scaled = [(self._s(x), self._s(y)) for x, y in points]
        self.draw.line(
            scaled, fill=value, width=max(1, round(self._s(width))), joint="curve"
        )
        for x, y in (scaled[0], scaled[-1]):
            r = self._s(width) / 2.0
            self.draw.ellipse([x - r, y - r, x + r, y + r], fill=value)

    def array(self) -> np.ndarray:
        """Downsampled coverage in 0..1 (box filter = exact area anti-aliasing)."""
        small = self.image.resize((self.width, self.height), Image.Resampling.BOX)
        return np.asarray(small, dtype=np.float64) / 255.0


@dataclass
class Canvas:
    """RGB + alpha float canvas onto which masks are painted with pigments."""

    rgb: np.ndarray
    alpha: np.ndarray
    margin: int
    period: int

    @classmethod
    def vellum(
        cls,
        width: int,
        height: int,
        margin: int,
        period: int,
        rng: np.random.Generator,
        base: np.ndarray = VELLUM,
        darkness: float = 1.0,
    ) -> Canvas:
        """Opaque canvas covered with periodic vellum aligned on ``margin``."""
        tile = vellum_tile(period, rng, base, darkness)
        return cls(
            tile_to(tile, height, width, margin),
            np.ones((height, width)),
            margin,
            period,
        )

    @classmethod
    def empty(cls, width: int, height: int, margin: int, period: int) -> Canvas:
        """Fully transparent canvas whose 9-slice centre starts at ``margin``."""
        return cls(
            np.zeros((height, width, 3)), np.zeros((height, width)), margin, period
        )

    def tiled(self, tile: np.ndarray) -> np.ndarray:
        """Spread a ``period``-periodic tile over the canvas, aligned on the 9-slice centre."""
        width, height = self.size
        return tile_to(tile, height, width, self.margin)

    def metal(
        self,
        rng: np.random.Generator,
        dark: np.ndarray = GOLD_DARK,
        light: np.ndarray = GOLD_LIGHT,
        sheen: float = 0.30,
    ) -> np.ndarray:
        """Tile-aligned burnished metal field."""
        return self.tiled(metal_tile(self.period, rng, dark, light, sheen))

    def flat(
        self, color: np.ndarray, rng: np.random.Generator, variation: float = 0.05
    ) -> np.ndarray:
        """Tile-aligned hand-laid pigment field."""
        return self.tiled(flat_tile(self.period, color, rng, variation))

    @property
    def size(self) -> tuple[int, int]:
        """(width, height)."""
        return self.rgb.shape[1], self.rgb.shape[0]

    def paint(
        self, mask: Mask | np.ndarray, color: np.ndarray, opacity: float = 1.0
    ) -> None:
        """Lay a pigment (flat colour or per-pixel field) through ``mask`` ("over" operator)."""
        coverage = (mask.array() if isinstance(mask, Mask) else mask) * opacity
        cov = coverage[..., None]
        new_alpha = coverage + self.alpha * (1.0 - coverage)
        blended = color * cov + self.rgb * (self.alpha * (1.0 - coverage))[..., None]
        self.rgb = np.where(
            new_alpha[..., None] > 1e-6,
            blended / np.maximum(new_alpha, 1e-6)[..., None],
            self.rgb,
        )
        self.alpha = new_alpha

    def shade(self, amount: np.ndarray) -> None:
        """Darken (positive) or lighten (negative) the colour by a per-pixel factor."""
        self.rgb = np.clip(self.rgb * (1.0 - amount[..., None]), 0.0, 1.0)

    def save(self, path: Path) -> None:
        """Write an 8-bit RGBA PNG."""
        rgba = np.dstack([self.rgb, self.alpha])
        Image.fromarray(
            np.round(np.clip(rgba, 0, 1) * 255).astype(np.uint8), "RGBA"
        ).save(path, optimize=True)


def edge_vignette(width: int, height: int, depth: float, strength: float) -> np.ndarray:
    """Darkening towards the outer edge (handled page edges), 0 inside."""
    yy, xx = np.mgrid[0:height, 0:width]
    dist = np.minimum.reduce([xx + 0.5, yy + 0.5, width - xx - 0.5, height - yy - 0.5])
    return strength * np.clip(1.0 - dist / depth, 0.0, 1.0) ** 2


def gilded(
    canvas: Canvas, mask: Mask, rng: np.random.Generator, outline: float = 0.6
) -> None:
    """Burnished gold through ``mask`` with a thin ink contour (as in manuscripts)."""
    width, height = canvas.size
    coverage = mask.array()
    if outline > 0:
        contour = np.clip(
            gaussian_filter(coverage, outline) * 1.9 - coverage * 1.1, 0.0, 1.0
        )
        canvas.paint(contour, INK, 0.75)
    canvas.paint(coverage, canvas.metal(rng))


def inked(canvas: Canvas, mask: Mask, opacity: float = 0.9) -> None:
    """Iron-gall ink line."""
    canvas.paint(mask, INK, opacity)


# --------------------------------------------------------------------------------------
# Ornaments
# --------------------------------------------------------------------------------------


def ivy_leaf(mask: Mask, center: Point, size: float, angle: float) -> None:
    """Three-lobed ivy leaf (the signature leaf of French 14th-century borders)."""
    cx, cy = center
    points: list[Point] = []
    for i in range(72):
        t = 2.0 * math.pi * i / 72
        # Three lobes plus a pointed tip along the leaf axis.
        radius = size * (
            0.62 + 0.30 * math.cos(3.0 * t) ** 2 + 0.18 * max(0.0, math.cos(t)) ** 6
        )
        local_x = radius * math.cos(t)
        local_y = radius * math.sin(t) * 0.92
        rx = local_x * math.cos(angle) - local_y * math.sin(angle)
        ry = local_x * math.sin(angle) + local_y * math.cos(angle)
        points.append((cx + rx, cy + ry))
    mask.polygon(points)


def quatrefoil(mask: Mask, center: Point, size: float, value: int = 255) -> None:
    """Four-lobed rosette."""
    cx, cy = center
    lobe = size * 0.36
    for dx, dy in ((0, -1), (1, 0), (0, 1), (-1, 0)):
        mask.disc(cx + dx * size * 0.34, cy + dy * size * 0.34, lobe, value)
    mask.disc(cx, cy, size * 0.3, value)


def lozenge(
    mask: Mask, center: Point, half_w: float, half_h: float, value: int = 255
) -> None:
    """Diamond (heraldic lozenge)."""
    cx, cy = center
    mask.polygon(
        [(cx, cy - half_h), (cx + half_w, cy), (cx, cy + half_h), (cx - half_w, cy)],
        value,
    )


def vine_band(
    canvas: Canvas,
    rng: np.random.Generator,
    margin: int,
    period: int,
    band_center: float,
    amplitude: float,
    leaf: float,
) -> None:
    """Ivy rinceau running round the four sides, periodic with ``period`` along each edge.

    The stem is a sine wave; leaves alternate gold / azure / gules on either side with a
    short curled tendril ending in a gold bezant.
    """
    width, height = canvas.size
    stem = Mask(width, height)
    gold, azure, gules, tendrils, bezants, veins = (
        Mask(width, height) for _ in range(6)
    )
    leaf_masks = [
        gold,
        azure,
        gold,
        gules,
    ]  # 4 leaves per tile: the colour cycle repeats with it
    waves = 2  # full sine periods per tile

    def edge_points(
        offset: float, along_len: float
    ) -> list[tuple[float, float, float]]:
        """(along, across, slope) samples of the stem for one side."""
        samples = []
        steps = int(along_len * 2)
        for i in range(steps + 1):
            along = i / 2.0
            phase = 2.0 * math.pi * waves * (along - margin) / period
            across = offset + amplitude * math.sin(phase)
            slope = amplitude * math.cos(phase) * 2.0 * math.pi * waves / period
            samples.append((along, across, slope))
        return samples

    sides = [
        lambda a, c: (a, c),  # top
        lambda a, c: (a, height - c),  # bottom
        lambda a, c: (c, a),  # left
        lambda a, c: (width - c, a),  # right
    ]
    for side_index, to_xy in enumerate(sides):
        length = width if side_index < 2 else height
        samples = edge_points(band_center, length)
        stem.line([to_xy(a, c) for a, c, _ in samples], 1.5)
        # Leaves at the crests; periodic positions so the tile repeats.
        leaves_per_tile = waves * 2
        spacing = period / leaves_per_tile
        start = margin - period  # extend beyond the tile so corners are covered too
        pos = start + spacing * 0.5
        while pos < length + period:
            phase = 2.0 * math.pi * waves * (pos - margin) / period
            side_sign = 1.0 if math.sin(phase) >= 0 else -1.0
            across = band_center + amplitude * math.sin(phase)
            base = to_xy(pos, across)
            tip_across = across - side_sign * leaf * 1.25
            tip = to_xy(pos + leaf * 0.35, tip_across)
            angle = math.atan2(tip[1] - base[1], tip[0] - base[0])
            index = math.floor((pos - margin) / spacing) % len(leaf_masks)
            ivy_leaf(leaf_masks[index], tip, leaf, angle)
            stem.line([base, tip], 1.0)
            if index % 2 == 1:
                direction = (math.cos(angle), math.sin(angle))
                veins.line(
                    [
                        (
                            tip[0] - direction[0] * leaf * 0.45,
                            tip[1] - direction[1] * leaf * 0.45,
                        ),
                        (
                            tip[0] + direction[0] * leaf * 0.55,
                            tip[1] + direction[1] * leaf * 0.55,
                        ),
                    ],
                    0.5,
                )
            # Curled tendril on the opposite side, ending in a bezant.
            curl = []
            for j in range(10):
                t = j / 9.0
                curl_along = pos - leaf * 0.2 + t * leaf * 1.3
                curl_across = across + side_sign * (
                    t * leaf * 1.1 - 0.35 * leaf * math.sin(t * math.pi)
                )
                curl.append(to_xy(curl_along, curl_across))
            tendrils.line(curl, 0.6)
            end = curl[-1]
            bezants.disc(end[0], end[1], max(1.4, leaf * 0.28))
            pos += spacing
    inked(canvas, stem, 0.85)
    inked(canvas, tendrils, 0.7)
    gilded(canvas, gold, rng)
    for leaf_mask, color in ((azure, AZURE), (gules, GULES)):
        contour = np.clip(
            gaussian_filter(leaf_mask.array(), 0.6) * 1.9 - leaf_mask.array() * 1.1,
            0.0,
            1.0,
        )
        canvas.paint(contour, INK, 0.7)
        canvas.paint(leaf_mask, canvas.flat(color, rng))
    canvas.paint(veins, WHITE_LEAD, 0.8)
    gilded(canvas, bezants, rng, 0.5)


def bar_segments(
    canvas: Canvas,
    rng: np.random.Generator,
    x0: float,
    y0: float,
    x1: float,
    y1: float,
    segment: float,
    horizontal: bool,
    phase_origin: float,
) -> None:
    """Gold-edged "baguette": alternating azure/gules segments with white filigree dots."""
    width, height = canvas.size
    edge = Mask(width, height)
    azure = Mask(width, height)
    gules = Mask(width, height)
    dots = Mask(width, height)
    edge.rect(x0, y0, x1, y1)
    inset = 1.3
    along0, along1 = (x0, x1) if horizontal else (y0, y1)
    first = phase_origin + math.floor((along0 - phase_origin) / segment) * segment
    pos = first
    while pos < along1:
        index = int(round((pos - phase_origin) / segment)) % 2
        target = azure if index == 0 else gules
        a0 = max(pos + 0.6, along0)
        a1 = min(pos + segment - 0.6, along1)
        if a1 > a0:
            if horizontal:
                target.rect(a0, y0 + inset, a1, y1 - inset)
                dots.disc((a0 + a1) / 2.0, (y0 + y1) / 2.0, 0.75)
            else:
                target.rect(x0 + inset, a0, x1 - inset, a1)
                dots.disc((x0 + x1) / 2.0, (a0 + a1) / 2.0, 0.75)
        pos += segment
    gilded(canvas, edge, rng, 0.5)
    canvas.paint(azure, canvas.flat(AZURE, rng))
    canvas.paint(gules, canvas.flat(GULES, rng))
    canvas.paint(dots, WHITE_LEAD, 0.9)


def corner_boss(
    canvas: Canvas,
    rng: np.random.Generator,
    center: Point,
    size: float,
    field: np.ndarray = AZURE,
) -> None:
    """Gilded square boss with a coloured field and a gold quatrefoil (border corners)."""
    width, height = canvas.size
    cx, cy = center
    half = size / 2.0
    frame = Mask(width, height)
    frame.rect(cx - half, cy - half, cx + half, cy + half)
    gilded(canvas, frame, rng)
    inner = Mask(width, height)
    inner.rect(
        cx - half + size * 0.16,
        cy - half + size * 0.16,
        cx + half - size * 0.16,
        cy + half - size * 0.16,
    )
    canvas.paint(inner, canvas.flat(field, rng))
    flower = Mask(width, height)
    quatrefoil(flower, center, size * 0.52)
    gilded(canvas, flower, rng, 0.4)
    pip = Mask(width, height)
    pip.disc(cx, cy, size * 0.08)
    canvas.paint(pip, GULES if field is AZURE else AZURE)


# --------------------------------------------------------------------------------------
# Kit pieces
# --------------------------------------------------------------------------------------


@dataclass
class Piece:
    """One exported texture and its 9-slice metadata."""

    name: str
    margins: tuple[int, int, int, int]  # left, top, right, bottom (texture margins)
    content: tuple[int, int, int, int]  # suggested content margins
    tile: bool = True


def panel(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Default panel: vellum page, ink rule, gold band, vermilion filet, gilded corner bosses."""
    margin, period = 20, 128
    size = 2 * margin + period
    canvas = Canvas.vellum(size, size, margin, period, rng)
    canvas.shade(edge_vignette(size, size, 14.0, 0.18))
    outer = Mask(size, size)
    outer.frame(1, 1.4)
    inked(canvas, outer)
    band = Mask(size, size)
    band.frame(4, 3.2)
    gilded(canvas, band, rng)
    filet = Mask(size, size)
    filet.frame(8.6, 0.9)
    canvas.paint(filet, GULES, 0.85)
    for cx in (5.6, size - 5.6):
        for cy in (5.6, size - 5.6):
            corner_boss(canvas, rng, (cx, cy), 10.0)
    return canvas, Piece("panel", (margin, margin, margin, margin), (18, 14, 18, 15))


def panel_illuminated(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Large window: gold frame, ivy rinceau band, azure/gules bar, big corner bosses."""
    margin, period = 38, 128
    size = 2 * margin + period
    canvas = Canvas.vellum(size, size, margin, period, rng)
    canvas.shade(edge_vignette(size, size, 18.0, 0.16))
    outer = Mask(size, size)
    outer.frame(1, 1.4)
    inked(canvas, outer)
    band = Mask(size, size)
    band.frame(3.5, 3.0)
    gilded(canvas, band, rng)
    vine_band(canvas, rng, margin, period, band_center=17.5, amplitude=4.5, leaf=5.6)
    # Inner baguette on the four sides.
    b0, b1 = 29.0, 33.5
    bar_segments(canvas, rng, b0, b0, size - b0, b1, 16.0, True, margin)
    bar_segments(canvas, rng, b0, size - b1, size - b0, size - b0, 16.0, True, margin)
    bar_segments(canvas, rng, b0, b0, b1, size - b0, 16.0, False, margin)
    bar_segments(canvas, rng, size - b1, b0, size - b0, size - b0, 16.0, False, margin)
    for cx in (b0 + 2.5, size - b0 - 2.5):
        for cy in (b0 + 2.5, size - b0 - 2.5):
            corner_boss(canvas, rng, (cx, cy), 11.0, GULES)
    for cx in (13.0, size - 13.0):
        for cy in (13.0, size - 13.0):
            corner_boss(canvas, rng, (cx, cy), 19.0)
    return canvas, Piece(
        "panel_illuminated", (margin, margin, margin, margin), (44, 40, 44, 40)
    )


def top_bar(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """HUD band: vellum strip, gold top rule, azure/gules baguette and hanging ivy below."""
    left, right, period = 18, 18, 128
    top, body, bottom = 6, 36, 22
    width = left + period + right
    height = top + body + bottom
    canvas = Canvas.empty(width, height, left, period)
    page = Canvas.vellum(width, height, left, period, rng)
    body_mask = Mask(width, height)
    body_mask.rect(0, 0, width, top + body + 9)
    canvas.paint(body_mask, page.rgb)
    rule = Mask(width, height)
    rule.rect(0, 1, width, 3)
    gilded(canvas, rule, rng, 0.4)
    bar_segments(
        canvas, rng, -8, top + body, width + 8, top + body + 7, 16.0, True, left
    )
    ink = Mask(width, height)
    ink.rect(0, top + body + 7, width, top + body + 8.2)
    inked(canvas, ink)
    # Hanging ivy: short stems below the bar every 32 px, alternating colours.
    stems = Mask(width, height)
    gold, azure, gules = (Mask(width, height) for _ in range(3))
    bezants = Mask(width, height)
    leaves = [gold, gules, gold, azure]
    for i in range(-1, period // 32 + 2):
        x = left + 16 + i * 32
        y = top + body + 8
        sway = 3.0 if i % 2 == 0 else -3.0
        stems.line([(x, y), (x + sway * 0.4, y + 5), (x + sway, y + 8)], 0.9)
        ivy_leaf(leaves[i % 4], (x + sway, y + 10.5), 3.6, math.pi / 2)
        bezants.disc(x - sway * 1.6, y + 4.5, 1.3)
        stems.line([(x, y + 1), (x - sway * 1.2, y + 3.5)], 0.6)
    inked(canvas, stems, 0.8)
    gilded(canvas, gold, rng)
    canvas.paint(azure, canvas.flat(AZURE, rng))
    canvas.paint(gules, canvas.flat(GULES, rng))
    gilded(canvas, bezants, rng, 0.4)
    return canvas, Piece(
        "top_bar", (left, top, right, bottom), (14, top + 2, 14, bottom - 2)
    )


def button(rng: np.random.Generator, state: str) -> tuple[Canvas, Piece]:
    """Cartouche button: normal, hover, pressed (gilded leaf, azure filet), disabled."""
    margin, period = 9, 64
    size = 2 * margin + period
    base = {
        "normal": VELLUM_DARK,
        "hover": VELLUM,
        "pressed": np.array([0.960, 0.845, 0.560]),
        "disabled": np.array([0.82, 0.79, 0.72]),
    }[state]
    canvas = Canvas.vellum(
        size, size, margin, period, rng, base, 0.2 if state == "disabled" else 0.8
    )
    if state == "pressed":
        yy, xx = np.mgrid[0:size, 0:size]
        inner = np.clip(1.0 - np.minimum(yy, xx) / 7.0, 0.0, 1.0) ** 2
        canvas.shade(0.22 * inner)  # pressed into the page: shadow on top-left edges
    else:
        canvas.shade(edge_vignette(size, size, 7.0, 0.22 if state != "hover" else 0.10))
    outer = Mask(size, size)
    outer.frame(0, 1.2)
    inked(canvas, outer, 0.45 if state == "disabled" else 0.9)
    if state != "disabled":
        band = Mask(size, size)
        band.frame(2.2, 2.2 if state in ("hover", "pressed") else 1.6)
        gilded(canvas, band, rng, 0.0)
        for cx in (3.2, size - 3.2):
            for cy in (3.2, size - 3.2):
                dot = Mask(size, size)
                lozenge(dot, (cx, cy), 3.0, 3.0)
                canvas.paint(
                    dot, canvas.flat(AZURE if state == "pressed" else GULES, rng)
                )
    if state == "pressed":
        filet = Mask(size, size)
        filet.frame(5.4, 1.2)
        canvas.paint(filet, canvas.flat(AZURE, rng), 0.9)
    return canvas, Piece(
        f"button_{state}", (margin, margin, margin, margin), (12, 5, 12, 5)
    )


def tab(rng: np.random.Generator, selected: bool) -> tuple[Canvas, Piece]:
    """Book-tab: selected = light vellum with a vermilion head, unselected = darker leaf."""
    margin, period = 8, 64
    size = 2 * margin + period
    base = VELLUM if selected else VELLUM_DARK * 0.96
    canvas = Canvas.vellum(size, size, margin, period, rng, base)
    canvas.shade(edge_vignette(size, size, 6.0, 0.12 if selected else 0.24))
    outer = Mask(size, size)
    outer.rect(0, 0, size, 1.2)
    outer.rect(0, 0, 1.2, size)
    outer.rect(size - 1.2, 0, size, size)
    inked(canvas, outer, 0.85)
    head = Mask(size, size)
    head.rect(1.2, 1.2, size - 1.2, 4.0 if selected else 2.4)
    if selected:
        canvas.paint(head, canvas.flat(GULES, rng))
        gold = Mask(size, size)
        gold.rect(1.2, 4.0, size - 1.2, 5.4)
        gilded(canvas, gold, rng, 0.0)
    else:
        gilded(canvas, head, rng, 0.0)
    return canvas, Piece(
        "tab_selected" if selected else "tab_unselected",
        (margin, margin, margin, margin),
        (12, 6, 12, 5),
    )


def tooltip(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Marginal note: light vellum, gold and vermilion double filet, lozenge corners."""
    margin, period = 12, 96
    size = 2 * margin + period
    canvas = Canvas.vellum(
        size, size, margin, period, rng, np.array([0.965, 0.925, 0.815])
    )
    canvas.shade(edge_vignette(size, size, 8.0, 0.14))
    outer = Mask(size, size)
    outer.frame(0, 1.2)
    inked(canvas, outer)
    band = Mask(size, size)
    band.frame(2.4, 1.8)
    gilded(canvas, band, rng, 0.0)
    filet = Mask(size, size)
    filet.frame(5.6, 0.9)
    canvas.paint(filet, GULES, 0.85)
    for cx in (3.3, size - 3.3):
        for cy in (3.3, size - 3.3):
            dot = Mask(size, size)
            lozenge(dot, (cx, cy), 3.6, 3.6)
            canvas.paint(dot, canvas.flat(AZURE, rng))
    return canvas, Piece("tooltip", (margin, margin, margin, margin), (11, 8, 11, 8))


def inset(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Writing field (LineEdit, lists, tab pages): pale vellum with a ruled ink edge."""
    margin, period = 6, 96
    size = 2 * margin + period
    canvas = Canvas.vellum(
        size, size, margin, period, rng, np.array([0.975, 0.945, 0.860]), 0.5
    )
    yy, xx = np.mgrid[0:size, 0:size]
    inner_shadow = 0.10 * np.clip(1.0 - np.minimum(yy, xx) / 5.0, 0, 1)
    canvas.shade(inner_shadow)
    outer = Mask(size, size)
    outer.frame(0, 1.0)
    inked(canvas, outer, 0.7)
    return canvas, Piece("inset", (margin, margin, margin, margin), (8, 4, 8, 4))


def bar_strip(
    rng: np.random.Generator,
    name: str,
    dark: np.ndarray,
    light: np.ndarray,
    horizontal: bool,
) -> tuple[Canvas, Piece]:
    """Rounded pigment bar for sliders and scroll grabbers."""
    margin, period, thick = 5, 32, 10
    length = 2 * margin + period
    width, height = (length, thick) if horizontal else (thick, length)
    canvas = Canvas.empty(width, height, margin, period)
    shape = Mask(width, height)
    radius = thick / 2.0 - 0.5
    if horizontal:
        shape.rect(radius, 0.5, width - radius, height - 0.5)
        shape.disc(radius + 0.5, height / 2, radius)
        shape.disc(width - radius - 0.5, height / 2, radius)
    else:
        shape.rect(0.5, radius, width - 0.5, height - radius)
        shape.disc(width / 2, radius + 0.5, radius)
        shape.disc(width / 2, height - radius - 0.5, radius)
    contour = np.clip(
        gaussian_filter(shape.array(), 0.6) * 1.9 - shape.array() * 1.1, 0, 1
    )
    canvas.paint(contour, INK, 0.8)
    canvas.paint(shape, canvas.metal(rng, dark, light, 0.35))
    margins = (margin, 0, margin, 0) if horizontal else (0, margin, 0, margin)
    return canvas, Piece(name, margins, (2, 3, 2, 3))


def slider_track(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Engraved slider groove."""
    canvas, piece = bar_strip(
        rng, "slider_track", VELLUM_DARK * 0.85, VELLUM * 0.98, True
    )
    return canvas, piece


# --------------------------------------------------------------------------------------
# DN ui-kit: primary button, ornate tabs, progress, slider grabber, separators, frames
# --------------------------------------------------------------------------------------

AZURE_DEEP = np.array([0.075, 0.140, 0.390])
AZURE_GREY = np.array([0.420, 0.450, 0.560])


def _solid(canvas: Canvas, color: np.ndarray, rng: np.random.Generator) -> None:
    """Cover the whole canvas with a hand-laid pigment field."""
    width, height = canvas.size
    full = Mask(width, height)
    full.rect(0, 0, width, height)
    canvas.paint(full, canvas.flat(color, rng, 0.06))


def button_primary(rng: np.random.Generator, state: str) -> tuple[Canvas, Piece]:
    """Strong action button: azure field, gold band, white-lead filet, vermilion lozenges."""
    margin, period = 10, 64
    size = 2 * margin + period
    field = {
        "normal": AZURE,
        "hover": AZURE_LIGHT,
        "pressed": AZURE_DEEP,
        "disabled": AZURE_GREY,
    }[state]
    canvas = Canvas.empty(size, size, margin, period)
    _solid(canvas, field, rng)
    if state == "pressed":
        yy, xx = np.mgrid[0:size, 0:size]
        canvas.shade(0.25 * np.clip(1.0 - np.minimum(yy, xx) / 8.0, 0.0, 1.0) ** 2)
    else:
        canvas.shade(edge_vignette(size, size, 8.0, 0.20))
    outer = Mask(size, size)
    outer.frame(0, 1.3)
    inked(canvas, outer, 0.5 if state == "disabled" else 0.95)
    band = Mask(size, size)
    band.frame(1.8, 2.6 if state in ("hover", "pressed") else 2.0)
    name = f"button_primary_{state}"
    if state == "disabled":
        canvas.paint(band, canvas.metal(rng, VELLUM_DARK * 0.8, VELLUM * 0.9), 0.7)
        return canvas, Piece(name, (margin,) * 4, (14, 6, 14, 6))
    gilded(canvas, band, rng, 0.0)
    filet = Mask(size, size)
    filet.frame(5.2, 0.9)
    canvas.paint(filet, WHITE_LEAD, 0.85 if state != "pressed" else 0.55)
    for cx in (4.0, size - 4.0):
        for cy in (4.0, size - 4.0):
            dot = Mask(size, size)
            lozenge(dot, (cx, cy), 3.4, 3.4)
            gilded(canvas, dot, rng, 0.3)
            pip = Mask(size, size)
            lozenge(pip, (cx, cy), 1.7, 1.7)
            canvas.paint(pip, canvas.flat(GULES, rng))
    return canvas, Piece(name, (margin,) * 4, (14, 6, 14, 6))


def tab_ornate(rng: np.random.Generator, selected: bool) -> tuple[Canvas, Piece]:
    """Gilded book-tab: active = bright vellum, gold head, gules tongue, azure hairline."""
    margin, period = 10, 64
    size = 2 * margin + period
    base = VELLUM if selected else VELLUM_DARK * 0.93
    canvas = Canvas.vellum(size, size, margin, period, rng, base)
    canvas.shade(edge_vignette(size, size, 7.0, 0.10 if selected else 0.26))
    side = Mask(size, size)
    side.rect(0, 0, size, 1.2)
    side.rect(0, 0, 1.2, size)
    side.rect(size - 1.2, 0, size, size)
    inked(canvas, side, 0.9 if selected else 0.7)
    head = Mask(size, size)
    head.rect(1.2, 1.2, size - 1.2, 5.0 if selected else 3.0)
    gilded(canvas, head, rng, 0.0)
    if selected:
        tongue = Mask(size, size)
        tongue.rect(1.2, 5.0, size - 1.2, 7.0)
        canvas.paint(tongue, canvas.flat(GULES, rng))
        hair = Mask(size, size)
        hair.rect(3.4, 8.6, size - 3.4, 9.4)
        canvas.paint(hair, canvas.flat(AZURE, rng), 0.8)
        for cx in (3.4, size - 3.4):
            dot = Mask(size, size)
            lozenge(dot, (cx, 3.1), 1.9, 1.9)
            canvas.paint(dot, canvas.flat(AZURE, rng))
    name = "tab_ornate_selected" if selected else "tab_ornate_unselected"
    return canvas, Piece(name, (margin,) * 4, (14, 8, 14, 5))


def progress_frame(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Gauge trough: ink-ruled vellum groove bordered by a gold hairline (horizontal 9-slice)."""
    margin, period, thick = 6, 32, 16
    width, height = 2 * margin + period, thick
    canvas = Canvas.vellum(width, height, margin, period, rng, VELLUM_DARK * 0.93, 0.6)
    yy = np.mgrid[0:height, 0:width][0]
    canvas.shade(0.16 * np.clip(1.0 - yy / 5.0, 0.0, 1.0))  # inner shadow on top
    ink = Mask(width, height)
    ink.rect(0, 0, width, 1.2)
    ink.rect(0, height - 1.2, width, height)
    ink.rect(0, 0, 1.2, height)
    ink.rect(width - 1.2, 0, width, height)
    inked(canvas, ink, 0.9)
    gold = Mask(width, height)
    gold.rect(1.2, 1.2, width - 1.2, 2.4)
    gold.rect(1.2, height - 2.4, width - 1.2, height - 1.2)
    gilded(canvas, gold, rng, 0.0)
    return canvas, Piece("progress_frame", (margin, 0, margin, 0), (3, 4, 3, 4))


def progress_fill(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Gauge fill: vermilion pigment capped by a gold highlight and white-lead dots."""
    margin, period, thick = 5, 32, 10
    width, height = 2 * margin + period, thick
    canvas = Canvas.empty(width, height, margin, period)
    shape = Mask(width, height)
    shape.rect(0, 0.5, width, height - 0.5)
    canvas.paint(shape, canvas.flat(GULES, rng))
    sheen = Mask(width, height)
    sheen.rect(0, 1.0, width, 2.6)
    canvas.paint(sheen, canvas.metal(rng, GOLD_DARK, GOLD_LIGHT, 0.4), 0.9)
    dots = Mask(width, height)
    for i in range(period // 8):
        dots.disc(margin + 4 + i * 8, height * 0.68, 0.7)
    canvas.paint(dots, WHITE_LEAD, 0.8)
    edge = Mask(width, height)
    edge.rect(0, 0, width, 0.7)
    edge.rect(0, height - 0.7, width, height)
    inked(canvas, edge, 0.6)
    return canvas, Piece("progress_fill", (margin, 0, margin, 0), (0, 3, 0, 3))


def slider_grabber(
    rng: np.random.Generator, hover: bool = False
) -> tuple[Canvas, Piece]:
    """Slider handle: a wax cabochon rimmed with gold (plain icon, not a 9-slice)."""
    size = 22
    canvas = Canvas.empty(size, size, 0, size)
    centre = size / 2.0
    rim = Mask(size, size)
    rim.disc(centre, centre, 10.0)
    gilded(canvas, rim, rng, 0.8)
    wax = Mask(size, size)
    wax.disc(centre, centre, 7.4)
    wax_color = np.clip(GULES * (1.18 if hover else 1.0), 0.0, 1.0)
    canvas.paint(wax, canvas.flat(wax_color, rng, 0.04))
    yy, xx = np.mgrid[0:size, 0:size]
    light = np.exp(-(((xx - centre + 2.4) ** 2 + (yy - centre + 2.4) ** 2) / 9.0))
    canvas.paint(light * wax.array(), WHITE_LEAD, 0.5)
    ring = Mask(size, size)
    ring.disc(centre, centre, 7.4)
    ring.disc(centre, centre, 6.4, 0)
    inked(canvas, ring, 0.35)
    name = "slider_grabber_hover" if hover else "slider_grabber"
    return canvas, Piece(name, (0, 0, 0, 0), (0, 0, 0, 0), tile=False)


def separator_h(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Section rule: gold hairline ending in quatrefoil-and-lozenge fleurons."""
    margin, period, height = 18, 32, 12
    width = 2 * margin + period
    canvas = Canvas.empty(width, height, margin, period)
    centre = height / 2.0
    line = Mask(width, height)
    line.rect(0, centre - 0.7, width, centre + 0.7)
    gilded(canvas, line, rng, 0.0)
    hair = Mask(width, height)
    hair.rect(0, centre + 2.0, width, centre + 2.6)
    canvas.paint(hair, canvas.flat(GULES, rng), 0.6)
    for cx in (7.0, width - 7.0):
        flower = Mask(width, height)
        quatrefoil(flower, (cx, centre), 8.0)
        gilded(canvas, flower, rng, 0.5)
        pip = Mask(width, height)
        lozenge(pip, (cx, centre), 1.6, 1.6)
        canvas.paint(pip, canvas.flat(AZURE, rng))
        tip = Mask(width, height)
        lozenge(tip, (cx + (8.5 if cx < margin else -8.5), centre), 2.2, 2.2)
        canvas.paint(tip, canvas.flat(GULES, rng))
    return canvas, Piece("separator_h", (margin, 0, margin, 0), (0, 0, 0, 0))


def inset_ornate(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Framed reading field: pale vellum, ink rule, gold band, vermilion filet, lozenges."""
    margin, period = 10, 96
    size = 2 * margin + period
    canvas = Canvas.vellum(
        size, size, margin, period, rng, np.array([0.975, 0.945, 0.860]), 0.5
    )
    yy, xx = np.mgrid[0:size, 0:size]
    canvas.shade(0.10 * np.clip(1.0 - np.minimum(yy, xx) / 7.0, 0, 1))
    outer = Mask(size, size)
    outer.frame(0, 1.1)
    inked(canvas, outer, 0.85)
    band = Mask(size, size)
    band.frame(2.2, 1.4)
    gilded(canvas, band, rng, 0.0)
    filet = Mask(size, size)
    filet.frame(4.6, 0.8)
    canvas.paint(filet, GULES, 0.8)
    for cx in (3.3, size - 3.3):
        for cy in (3.3, size - 3.3):
            dot = Mask(size, size)
            lozenge(dot, (cx, cy), 3.0, 3.0)
            canvas.paint(dot, canvas.flat(AZURE, rng))
    return canvas, Piece("inset_ornate", (margin,) * 4, (12, 8, 12, 8))


def initial_frame(rng: np.random.Generator) -> tuple[Canvas, Piece]:
    """Frame for a drop cap: azure field, gold border, vermilion corner pips.

    The letter itself is set in the theme font over this field (none is drawn here).
    """
    margin, period = 10, 24
    size = 2 * margin + period
    canvas = Canvas.empty(size, size, margin, period)
    _solid(canvas, AZURE, rng)
    canvas.shade(edge_vignette(size, size, 6.0, 0.18))
    outer = Mask(size, size)
    outer.frame(0, 1.2)
    inked(canvas, outer)
    band = Mask(size, size)
    band.frame(1.4, 2.6)
    gilded(canvas, band, rng, 0.0)
    dots = Mask(size, size)
    dots.frame(5.6, 0.7)
    canvas.paint(dots, WHITE_LEAD, 0.8)
    for cx in (margin * 0.5 + 1.0, size - margin * 0.5 - 1.0):
        for cy in (margin * 0.5 + 1.0, size - margin * 0.5 - 1.0):
            pip = Mask(size, size)
            lozenge(pip, (cx, cy), 2.4, 2.4)
            canvas.paint(pip, canvas.flat(GULES, rng))
    return canvas, Piece("initial_frame", (margin,) * 4, (8, 6, 8, 6))


def build(output_dir: Path = OUTPUT_DIR, seed: int = 1337) -> list[Path]:
    """Render the whole kit into ``output_dir`` and write the ``kit.json`` sidecar."""
    output_dir.mkdir(parents=True, exist_ok=True)
    makers: list[Callable[[np.random.Generator], tuple[Canvas, Piece]]] = [
        panel,
        panel_illuminated,
        top_bar,
        tooltip,
        inset,
        slider_track,
        lambda rng: bar_strip(rng, "gold_bar_h", GOLD_DARK, GOLD_LIGHT, True),
        lambda rng: bar_strip(rng, "gold_bar_v", GOLD_DARK, GOLD_LIGHT, False),
        lambda rng: bar_strip(rng, "azure_bar_h", AZURE, AZURE_LIGHT, True),
        lambda rng: tab(rng, True),
        lambda rng: tab(rng, False),
    ]
    makers += [
        lambda rng, s=state: button(rng, s)
        for state in ("normal", "hover", "pressed", "disabled")
    ]
    # DN ui-kit pieces are appended last: per-piece seeds depend on the index, so the
    # historical textures above stay byte-identical.
    makers += [
        lambda rng, s=state: button_primary(rng, s)
        for state in ("normal", "hover", "pressed", "disabled")
    ]
    makers += [
        lambda rng: tab_ornate(rng, True),
        lambda rng: tab_ornate(rng, False),
        progress_frame,
        progress_fill,
        lambda rng: slider_grabber(rng, False),
        lambda rng: slider_grabber(rng, True),
        separator_h,
        inset_ornate,
        initial_frame,
    ]
    written: list[Path] = []
    sidecar: dict[str, dict] = {}
    for index, maker in enumerate(makers):
        rng = np.random.default_rng(seed + index * 7919)
        canvas, piece = maker(rng)
        path = output_dir / f"{piece.name}.png"
        canvas.save(path)
        width, height = canvas.size
        override = output_dir / "nb" / f"{piece.name}.png"
        if override.exists():
            with Image.open(override) as nb_image:
                if nb_image.size != (width, height):
                    raise ValueError(
                        f"{override} : taille {nb_image.size} != {(width, height)}"
                    )
            shutil.copyfile(override, path)
        written.append(path)
        sidecar[piece.name] = {
            "size": [width, height],
            "margins": list(piece.margins),
            "content": list(piece.content),
        }
    (output_dir / SIDECAR_NAME).write_text(
        json.dumps(sidecar, indent=2) + "\n", encoding="utf-8"
    )
    return written


def contact_sheet(out_path: Path, source_dir: Path = OUTPUT_DIR) -> Path:
    """Compose a review board of the DN ui-kit widgets (buttons, tabs, gauge, rules) on a dark ground."""
    names = [
        ["button_normal", "button_hover", "button_pressed", "button_disabled"],
        [f"button_primary_{s}" for s in ("normal", "hover", "pressed", "disabled")],
        [
            "tab_ornate_selected",
            "tab_ornate_unselected",
            "tab_selected",
            "tab_unselected",
        ],
        ["progress_frame", "progress_fill", "separator_h", "slider_track"],
        ["slider_grabber", "slider_grabber_hover", "inset_ornate", "initial_frame"],
    ]
    kit = json.loads((source_dir / SIDECAR_NAME).read_text(encoding="utf-8"))
    cell_w, cell_h, pad = 220, 90, 16
    sheet = Image.new(
        "RGBA",
        (len(names[0]) * (cell_w + pad) + pad, len(names) * (cell_h + pad) + pad),
        (36, 30, 24, 255),
    )
    for row, line in enumerate(names):
        for col, name in enumerate(line):
            with Image.open(source_dir / f"{name}.png") as image:
                tex = image.convert("RGBA")
            margins = kit[name]["margins"]
            if not any(margins):
                target = tex  # plain icon
            else:
                target = _nine_slice(
                    tex, margins, (cell_w, cell_h if margins[1] else tex.height)
                )
            x = pad + col * (cell_w + pad) + (cell_w - target.width) // 2
            y = pad + row * (cell_h + pad) + (cell_h - target.height) // 2
            sheet.alpha_composite(target, (x, y))
    out_path.parent.mkdir(parents=True, exist_ok=True)
    sheet.save(out_path)
    return out_path


def _nine_slice(
    tex: Image.Image, margins: list[int], size: tuple[int, int]
) -> Image.Image:
    """Stretch ``tex`` to ``size`` with 9-slice margins (left, top, right, bottom)."""
    left, top, right, bottom = margins
    width, height = size
    out = Image.new("RGBA", size, (0, 0, 0, 0))
    xs_src = [0, left, tex.width - right, tex.width]
    ys_src = [0, top, tex.height - bottom, tex.height]
    xs_dst = [0, left, width - right, width]
    ys_dst = [0, top, height - bottom, height]
    for i in range(3):
        for j in range(3):
            src = (xs_src[i], ys_src[j], xs_src[i + 1], ys_src[j + 1])
            dst = (xs_dst[i], ys_dst[j], xs_dst[i + 1], ys_dst[j + 1])
            if (
                src[2] <= src[0]
                or src[3] <= src[1]
                or dst[2] <= dst[0]
                or dst[3] <= dst[1]
            ):
                continue
            piece = tex.crop(src).resize(
                (dst[2] - dst[0], dst[3] - dst[1]), Image.Resampling.NEAREST
            )
            out.alpha_composite(piece, (dst[0], dst[1]))
    return out
