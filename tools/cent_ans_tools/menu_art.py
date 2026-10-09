"""Menu illustration: an old parchment map rendered from ``data/map/``.

The start menu and the loading screen show a stylised "portolan" map of the 1337 world,
drawn procedurally from the real map data (Pillow + numpy only):

* parchment base (multi-octave value noise, stains, burnt edges);
* sea slightly cooled, with ripple lines following the coast (contours of a blurred land
  mask), faint rhumb lines radiating from a compass rose placed in open sea;
* land with an ink wash hillshade from ``heightmap.png`` and relief hachures (short strokes
  along the slope), molehill mountain glyphs on the high ground;
* inked coastlines (``coastline.geojson``), rivers (``rivers.geojson``), dotted province
  borders (``provinces.geojson``) and the capitals of the factions (``data/factions``,
  ``capital_city``) with their names;
* a title cartouche « Cent Ans » and a graduated double frame.

Vector layers are drawn at 2x then downsampled (anti-aliasing). The output is
deterministic for a given machine (same data, Pillow version and fonts: the first serif
font found in :data:`FONT_CANDIDATES`, Pillow's bundled font otherwise).

A small JSON sidecar gives the cartouche rectangle so that Godot can lay the menu out
around it.
"""

from __future__ import annotations

import json
import math
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont
from cent_ans_tools.paths import REPO_DIR

MAP_DIR = REPO_DIR / "data" / "map"
FACTIONS_DIR = REPO_DIR / "data" / "factions"
PROVINCES_DIR = REPO_DIR / "data" / "provinces"
UI_ASSETS_DIR = REPO_DIR / "game" / "assets" / "ui"
OUTPUT_NAME = "menu_map.jpg"
SIDECAR_NAME = "menu_map.json"

WIDTH = 2560
HEIGHT = 1440
SUPERSAMPLE = 2
SEED = 1337
JPEG_QUALITY = 90
HEIGHT_MIN_M = -200.0
HEIGHT_MAX_M = 4800.0

Color = tuple[int, int, int]
PARCHMENT: Color = (233, 216, 176)
PARCHMENT_DARK: Color = (160, 118, 70)
SEA: Color = (208, 206, 178)
INK: Color = (58, 38, 22)
RIVER_INK: Color = (64, 84, 104)
RHUMB_RED: Color = (150, 42, 30)
CITY_RED: Color = (156, 32, 26)
GOLD: Color = (176, 132, 52)

FONT_CANDIDATES = (
    "/System/Library/Fonts/Supplemental/Georgia.ttf",
    "/Library/Fonts/Georgia.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif.ttf",
    "/usr/share/fonts/dejavu/DejaVuSerif.ttf",
    "Georgia.ttf",
    "DejaVuSerif.ttf",
)
ITALIC_CANDIDATES = (
    "/System/Library/Fonts/Supplemental/Georgia Italic.ttf",
    "/Library/Fonts/Georgia Italic.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSerif-Italic.ttf",
    "Georgia Italic.ttf",
)


@dataclass(frozen=True)
class Frame:
    """Map-pixel window rendered into the output image."""

    x0: float
    y0: float
    scale: float  # output pixels per map pixel

    def to_out(self, x: float, y: float, factor: float = 1.0) -> tuple[float, float]:
        """Map pixel -> output pixel (times ``factor`` for the supersampled canvas)."""
        return (
            (x - self.x0) * self.scale * factor,
            (y - self.y0) * self.scale * factor,
        )

    def box(self, width: int, height: int) -> tuple[float, float, float, float]:
        """Map-pixel rectangle covered by a ``width`` x ``height`` output."""
        return (
            self.x0,
            self.y0,
            self.x0 + width / self.scale,
            self.y0 + height / self.scale,
        )


# --- Data -----------------------------------------------------------------------------


def _load_json(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _lines(geojson: dict) -> list[tuple[dict, list[list[float]]]]:
    """(properties, coordinates) of every LineString / ring of a FeatureCollection."""
    result: list[tuple[dict, list[list[float]]]] = []
    for feature in geojson.get("features", []):
        geometry = feature.get("geometry") or {}
        properties = feature.get("properties") or {}
        kind = geometry.get("type")
        coordinates = geometry.get("coordinates", [])
        if kind == "LineString":
            result.append((properties, coordinates))
        elif kind in ("MultiLineString", "Polygon"):
            result.extend((properties, line) for line in coordinates)
        elif kind == "MultiPolygon":
            for polygon in coordinates:
                result.extend((properties, ring) for ring in polygon)
    return result


def faction_capitals(
    factions_dir: Path = FACTIONS_DIR, provinces: dict | None = None
) -> list[tuple[str, tuple[float, float]]]:
    """(city name, map pixel) of each faction capital that has a ``capital_city``."""
    provinces = provinces or _load_json(MAP_DIR / "provinces.geojson")
    capital_px = {
        feature["properties"]["id"]: feature["properties"].get("capital_px")
        for feature in provinces["features"]
    }
    cities: dict[str, tuple[float, float]] = {}
    for path in sorted(factions_dir.glob("*.json")):
        faction = _load_json(path)
        city = faction.get("capital_city")
        position = capital_px.get(faction.get("capital", ""))
        if not city or not position:
            continue
        name = city
        if isinstance(city, dict):
            name = city.get("name", city.get("display", ""))
            if isinstance(name, dict):
                name = name.get("display", "")
        if name and name not in cities:
            cities[str(name)] = (float(position[0]), float(position[1]))
    return sorted(cities.items())


def playable_factions(factions_dir: Path = FACTIONS_DIR) -> set[str]:
    """Ids of the factions flagged ``playable`` in ``data/factions``."""
    return {
        str(data.get("id"))
        for data in (_load_json(path) for path in sorted(factions_dir.glob("*.json")))
        if data.get("playable")
    }


def choose_frame(
    provinces: dict, map_size: tuple[int, int], width: int, height: int
) -> Frame:
    """Window around the provinces of the playable factions, at the output aspect ratio.

    Falls back to every province when no owner is playable (fixtures).
    """
    playable = playable_factions()
    features = [
        feature
        for feature in provinces["features"]
        if feature["properties"].get("owner") in playable
    ] or provinces["features"]
    points = np.array(
        [
            feature["properties"].get("capital_px")
            or feature["properties"].get("centroid")
            for feature in features
        ],
        dtype=np.float64,
    )
    x_min, y_min = points.min(axis=0)
    x_max, y_max = points.max(axis=0)
    pad = 0.12 * max(x_max - x_min, y_max - y_min)
    x_min, y_min, x_max, y_max = x_min - pad, y_min - pad, x_max + pad, y_max + pad
    aspect = width / height
    box_w, box_h = x_max - x_min, y_max - y_min
    if box_w / box_h < aspect:
        box_w = box_h * aspect
    else:
        box_h = box_w / aspect
    box_w = min(box_w, map_size[0], map_size[1] * aspect)
    box_h = box_w / aspect
    center_x = (x_min + x_max) / 2.0
    center_y = (y_min + y_max) / 2.0
    x0 = float(np.clip(center_x - box_w / 2.0, 0.0, map_size[0] - box_w))
    y0 = float(np.clip(center_y - box_h / 2.0, 0.0, map_size[1] - box_h))
    return Frame(x0=x0, y0=y0, scale=width / box_w)


def _resample(image: Image.Image, frame: Frame, width: int, height: int) -> np.ndarray:
    """Crop ``image`` (map pixels) to the frame and resize it to the output (float32)."""
    box = tuple(round(value) for value in frame.box(width, height))
    return np.asarray(
        image.crop(box).resize((width, height), Image.Resampling.BILINEAR),
        dtype=np.float32,
    )


def load_rasters(
    map_dir: Path, frame: Frame, width: int, height: int
) -> tuple[np.ndarray, np.ndarray]:
    """Heights in metres and land coverage (0..1) at output resolution."""
    raw = np.asarray(Image.open(map_dir / "heightmap.png"), dtype=np.float32)
    metres = HEIGHT_MIN_M + raw / 65535.0 * (HEIGHT_MAX_M - HEIGHT_MIN_M)
    heights = _resample(Image.fromarray(metres, mode="F"), frame, width, height)
    mask = Image.open(map_dir / "land_mask.png").convert("L")
    land = _resample(mask, frame, width, height) / 255.0
    return heights, np.clip(land, 0.0, 1.0)


# --- Raster layers --------------------------------------------------------------------


def value_noise(
    shape: tuple[int, int], rng: np.random.Generator, octaves: int = 5
) -> np.ndarray:
    """Multi-octave value noise in [-1, 1] (bicubic upsampling of random grids)."""
    height, width = shape
    total = np.zeros(shape, dtype=np.float32)
    amplitude, norm = 1.0, 0.0
    for octave in range(octaves):
        cells = 3 * 2**octave
        grid = rng.random(
            (cells, max(2, round(cells * width / height))), dtype=np.float32
        )
        layer = Image.fromarray(grid, mode="F").resize(
            (width, height), Image.Resampling.BICUBIC
        )
        total += amplitude * (np.asarray(layer, dtype=np.float32) * 2.0 - 1.0)
        norm += amplitude
        amplitude *= 0.55
    return np.clip(total / norm, -1.0, 1.0)


def parchment(width: int, height: int, rng: np.random.Generator) -> np.ndarray:
    """Parchment colour field (H, W, 3) in [0, 255], with stains and burnt edges."""
    noise = value_noise((height, width), rng)
    fine = value_noise((height, width), rng, octaves=3)
    base = np.array(PARCHMENT, dtype=np.float32)
    dark = np.array(PARCHMENT_DARK, dtype=np.float32)
    shade = 0.10 + 0.12 * noise + 0.04 * fine
    # Burnt edges: darker toward the border (elliptic vignette, noisy).
    ys = np.linspace(-1.0, 1.0, height, dtype=np.float32)[:, None]
    xs = np.linspace(-1.0, 1.0, width, dtype=np.float32)[None, :]
    radial = np.sqrt(xs**2 * 0.9 + ys**2 * 1.1)
    shade += np.clip(radial - 0.72, 0.0, None) * (1.4 + 0.5 * noise)
    # A few water stains.
    stains = Image.new("L", (width, height), 0)
    draw = ImageDraw.Draw(stains)
    for _ in range(7):
        cx, cy = rng.random() * width, rng.random() * height
        radius = (0.03 + rng.random() * 0.07) * width
        draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=255)
    stains = stains.filter(ImageFilter.GaussianBlur(width * 0.02))
    ring = np.asarray(stains, dtype=np.float32) / 255.0
    shade += 0.06 * ring * (1.0 - ring) * 4.0
    shade = np.clip(shade, 0.0, 1.0)[..., None]
    return base * (1.0 - shade) + dark * shade


def hillshade(heights: np.ndarray, metres_per_px: float) -> np.ndarray:
    """Lambertian shade in [0, 1] (light from the north-west, relief exaggerated)."""
    dy, dx = np.gradient(heights, metres_per_px)
    exaggeration = 6.0
    slope = np.arctan(exaggeration * np.hypot(dx, dy))
    aspect = np.arctan2(-dx, dy)
    azimuth, altitude = math.radians(315.0), math.radians(40.0)
    shade = np.sin(altitude) * np.cos(slope) + np.cos(altitude) * np.sin(
        slope
    ) * np.cos(azimuth - aspect)
    return np.clip(shade, 0.0, 1.0).astype(np.float32)


def coast_ripples(land: np.ndarray, levels: tuple[float, ...]) -> np.ndarray:
    """Ripple lines in the sea (0..1): contours of a blurred land mask."""
    height, width = land.shape
    mask = Image.fromarray((land > 0.5).astype(np.uint8) * 255, mode="L")
    blurred = np.asarray(
        mask.filter(ImageFilter.GaussianBlur(max(2.0, width / 110.0))), dtype=np.float32
    )
    blurred /= 255.0
    lines = np.zeros_like(land)
    for rank, level in enumerate(levels):
        inside = Image.fromarray((blurred > level).astype(np.uint8) * 255, mode="L")
        edge = (
            np.asarray(inside.filter(ImageFilter.FIND_EDGES), dtype=np.float32) / 255.0
        )
        lines = np.maximum(lines, edge * (1.0 - rank / (len(levels) + 1.0)))
    lines *= land < 0.5
    softened = Image.fromarray((lines * 255).astype(np.uint8), mode="L").filter(
        ImageFilter.GaussianBlur(0.6)
    )
    return np.asarray(softened, dtype=np.float32) / 255.0 * 1.6


def _mix(base: np.ndarray, color: Color, alpha: np.ndarray) -> np.ndarray:
    alpha = np.clip(alpha, 0.0, 1.0)[..., None]
    return base * (1.0 - alpha) + np.array(color, dtype=np.float32) * alpha


# --- Vector layers --------------------------------------------------------------------


def _load_font(size: int, italic: bool = False) -> ImageFont.FreeTypeFont:
    for candidate in (ITALIC_CANDIDATES if italic else ()) + FONT_CANDIDATES:
        try:
            return ImageFont.truetype(candidate, size)
        except OSError:
            continue
    return ImageFont.load_default(size)


def _draw_polylines(
    draw: ImageDraw.ImageDraw,
    lines: list[list[tuple[float, float]]],
    color: tuple[int, int, int, int],
    width: float,
) -> None:
    for points in lines:
        if len(points) >= 2:
            draw.line(points, fill=color, width=max(1, round(width)), joint="curve")


def _project(frame: Frame, coordinates: list[list[float]], factor: float) -> list:
    return [frame.to_out(x, y, factor) for x, y in coordinates]


def draw_hachures(
    draw: ImageDraw.ImageDraw,
    heights: np.ndarray,
    land: np.ndarray,
    shade: np.ndarray,
    metres_per_px: float,
    factor: float,
    rng: np.random.Generator,
) -> None:
    """Short strokes along the slope, denser and darker on steep, shaded ground."""
    height, width = heights.shape
    spacing = max(4, round(width / 420))
    dy, dx = np.gradient(heights, metres_per_px)
    slope = np.hypot(dx, dy)
    ys, xs = np.mgrid[spacing // 2 : height : spacing, spacing // 2 : width : spacing]
    jitter = rng.random((2, *xs.shape)) - 0.5
    xs = np.clip(xs + jitter[0] * spacing, 0, width - 1)
    ys = np.clip(ys + jitter[1] * spacing, 0, height - 1)
    xi, yi = xs.astype(int), ys.astype(int)
    steep = slope[yi, xi]
    keep = (land[yi, xi] > 0.6) & (steep > 0.03) & (heights[yi, xi] > 150.0)
    for x, y, grade, gx, gy, light in zip(
        xs[keep],
        ys[keep],
        steep[keep],
        dx[yi[keep], xi[keep]],
        dy[yi[keep], xi[keep]],
        shade[yi[keep], xi[keep]],
        strict=True,
    ):
        length = min(1.0, grade / 0.08) * spacing * 1.1 + spacing * 0.25
        norm = math.hypot(gx, gy) or 1.0
        ux, uy = gx / norm, gy / norm
        alpha = int(min(1.0, 0.25 + grade / 0.10) * (1.15 - light) * 150)
        if alpha <= 8:
            continue
        start = (x * factor, y * factor)
        end = ((x - ux * length) * factor, (y - uy * length) * factor)
        draw.line((start, end), fill=(*INK, alpha), width=max(1, round(factor * 0.9)))


def draw_mountains(
    draw: ImageDraw.ImageDraw,
    heights: np.ndarray,
    land: np.ndarray,
    factor: float,
    rng: np.random.Generator,
) -> None:
    """Molehill glyphs on the high ground (> 1300 m), from back to front."""
    height, width = heights.shape
    spacing = max(10, round(width / 110))
    glyphs = []
    for gy in range(spacing // 2, height, spacing):
        for gx in range(spacing // 2, width, spacing):
            x = gx + (rng.random() - 0.5) * spacing * 0.8
            y = gy + (rng.random() - 0.5) * spacing * 0.6
            xi, yi = int(np.clip(x, 0, width - 1)), int(np.clip(y, 0, height - 1))
            elevation = float(heights[yi, xi])
            if elevation > 1300.0 and land[yi, xi] > 0.8:
                glyphs.append((y, x, elevation))
    glyphs.sort()
    for y, x, elevation in glyphs:
        size = spacing * (0.55 + min(1.0, (elevation - 1300.0) / 2200.0) * 0.6)
        half, top = size * 0.75, size
        base_y = y * factor
        left, peak, right = (
            ((x - half) * factor, base_y),
            (x * factor, (y - top) * factor),
            ((x + half) * factor, base_y),
        )
        draw.polygon((left, peak, right), fill=(*PARCHMENT, 235))
        # Shaded eastern flank: a few hatch lines.
        for step in range(1, 4):
            t = step / 4.0
            start = (
                peak[0] + (right[0] - peak[0]) * t,
                peak[1] + (right[1] - peak[1]) * t,
            )
            end = (x * factor + half * factor * t * 0.35, base_y)
            draw.line((start, end), fill=(*INK, 150), width=max(1, round(factor * 0.8)))
        draw.line(
            (left, peak, right), fill=(*INK, 230), width=max(1, round(factor * 1.2))
        )


def find_rose_center(
    land: np.ndarray, radius: float, avoid: tuple[float, float, float, float]
) -> tuple[float, float]:
    """Open-sea position for the compass rose (least land within ``radius``)."""
    height, width = land.shape
    best, best_score = (width * 0.15, height * 0.75), float("inf")
    for fy in np.linspace(0.25, 0.85, 13):
        for fx in np.linspace(0.08, 0.92, 22):
            cx, cy = fx * width, fy * height
            if avoid[0] - radius < cx < avoid[2] + radius and cy < avoid[3] + radius:
                continue
            x0, x1 = int(max(0, cx - radius)), int(min(width, cx + radius))
            y0, y1 = int(max(0, cy - radius)), int(min(height, cy + radius))
            score = float(land[y0:y1, x0:x1].mean()) + 0.02 * abs(fx - 0.2)
            if score < best_score:
                best, best_score = (cx, cy), score
    return best


def draw_rhumb_lines(
    draw: ImageDraw.ImageDraw, center: tuple[float, float], reach: float, factor: float
) -> None:
    """32 portolan lines radiating from the rose (the 8 winds a little stronger)."""
    cx, cy = center[0] * factor, center[1] * factor
    for index in range(32):
        angle = index * math.pi / 16.0
        strong = index % 4 == 0
        color = (*RHUMB_RED, 70) if strong else (*INK, 38)
        end = (
            cx + math.cos(angle) * reach * factor,
            cy + math.sin(angle) * reach * factor,
        )
        draw.line(
            ((cx, cy), end),
            fill=color,
            width=max(1, round(factor * (1.3 if strong else 0.8))),
        )


def draw_compass_rose(
    draw: ImageDraw.ImageDraw, center: tuple[float, float], radius: float, factor: float
) -> None:
    """Sixteen-point rose with two rings and a north mark."""
    cx, cy, r = center[0] * factor, center[1] * factor, radius * factor
    line = max(1, round(factor * 1.4))
    for ring in (1.0, 0.92):
        draw.ellipse(
            (cx - r * ring, cy - r * ring, cx + r * ring, cy + r * ring),
            outline=(*INK, 220),
            width=line,
        )
    for tick in range(64):
        angle = tick * math.pi / 32.0
        inner = 0.92 if tick % 4 else 0.86
        draw.line(
            (
                (cx + math.cos(angle) * r * inner, cy + math.sin(angle) * r * inner),
                (cx + math.cos(angle) * r, cy + math.sin(angle) * r),
            ),
            fill=(*INK, 200),
            width=max(1, round(factor * 0.9)),
        )
    points = [(16, 0.40, 0.05), (8, 0.62, 0.08), (4, 0.84, 0.11)]
    for count, length, width in points:
        for index in range(count):
            angle = -math.pi / 2.0 + index * 2.0 * math.pi / count
            tip = (cx + math.cos(angle) * r * length, cy + math.sin(angle) * r * length)
            side = angle + math.pi / 2.0
            left = (
                cx + math.cos(side) * r * width,
                cy + math.sin(side) * r * width,
            )
            right = (
                cx - math.cos(side) * r * width,
                cy - math.sin(side) * r * width,
            )
            dark = (*INK, 235) if count == 4 else (*RHUMB_RED, 225)
            draw.polygon((tip, left, (cx, cy)), fill=dark)
            draw.polygon((tip, right, (cx, cy)), fill=(*PARCHMENT, 245))
            draw.line((left, tip, right), fill=(*INK, 230), width=line)
    hub = r * 0.07
    draw.ellipse(
        (cx - hub, cy - hub, cx + hub, cy + hub),
        fill=(*GOLD, 255),
        outline=(*INK, 255),
        width=line,
    )
    font = _load_font(round(radius * 0.26 * factor))
    draw.text((cx, cy - r * 1.02), "N", font=font, fill=(*INK, 255), anchor="md")


def draw_city(
    draw: ImageDraw.ImageDraw,
    position: tuple[float, float],
    name: str,
    size: float,
    factor: float,
    font: ImageFont.FreeTypeFont,
) -> None:
    """Small crenellated town glyph with its name in italics, parchment halo."""
    x, y, s = position[0] * factor, position[1] * factor, size * factor
    line = max(1, round(factor * 1.1))
    draw.rectangle(
        (x - s, y - s * 0.4, x + s, y + s * 0.6),
        fill=(*CITY_RED, 255),
        outline=(*INK, 255),
        width=line,
    )
    for tower in (-1, 0, 1):
        tx = x + tower * s * 0.8
        top = y - s * (1.25 if tower == 0 else 0.95)
        draw.rectangle(
            (tx - s * 0.28, top, tx + s * 0.28, y - s * 0.4),
            fill=(*CITY_RED, 255),
            outline=(*INK, 255),
            width=line,
        )
    draw.text(
        (x + s * 1.6, y),
        name,
        font=font,
        fill=(*INK, 255),
        anchor="lm",
        stroke_width=max(1, round(factor * 2.2)),
        stroke_fill=(*PARCHMENT, 220),
    )


def cartouche_box(width: int, height: int) -> tuple[int, int, int, int]:
    """Title cartouche rectangle (output pixels): top right corner."""
    box_w, box_h = round(width * 0.30), round(height * 0.20)
    x0 = width - box_w - round(width * 0.035)
    y0 = round(height * 0.05)
    return (x0, y0, x0 + box_w, y0 + box_h)


def draw_cartouche(
    draw: ImageDraw.ImageDraw, box: tuple[int, int, int, int], factor: float
) -> None:
    """Parchment scroll with double border, corner roundels, title and subtitle."""
    x0, y0, x1, y1 = (value * factor for value in box)
    width, height = x1 - x0, y1 - y0
    line = max(1, round(factor * 2.0))
    draw.rectangle(
        (x0 + 8 * factor, y0 + 10 * factor, x1 + 8 * factor, y1 + 10 * factor),
        fill=(40, 26, 14, 70),
    )
    draw.rectangle(
        (x0, y0, x1, y1), fill=(*PARCHMENT, 250), outline=(*INK, 255), width=line * 2
    )
    inset = 10 * factor
    draw.rectangle(
        (x0 + inset, y0 + inset, x1 - inset, y1 - inset),
        outline=(*RHUMB_RED, 230),
        width=line,
    )
    for cx, cy in ((x0, y0), (x1, y0), (x0, y1), (x1, y1)):
        radius = 16 * factor
        draw.ellipse(
            (cx - radius, cy - radius, cx + radius, cy + radius),
            fill=(*GOLD, 255),
            outline=(*INK, 255),
            width=line,
        )
    title_font = _load_font(round(height * 0.40))
    draw.text(
        ((x0 + x1) / 2, y0 + height * 0.36),
        "Cent Ans",
        font=title_font,
        fill=(*INK, 255),
        anchor="mm",
    )
    lines = ("Carte des royaumes de France et d'Angleterre", "l'an de grâce MCCCXXXVII")
    size = round(height * 0.10)
    subtitle_font = _load_font(size, italic=True)
    while size > 8 and subtitle_font.getlength(lines[0]) > width * 0.84:
        size -= 1
        subtitle_font = _load_font(size, italic=True)
    for index, text in enumerate(lines):
        draw.text(
            ((x0 + x1) / 2, y0 + height * (0.70 + 0.13 * index)),
            text,
            font=subtitle_font,
            fill=(*INK, 235),
            anchor="mm",
        )
    rule_y = y0 + height * 0.59
    draw.line(
        (x0 + width * 0.18, rule_y, x1 - width * 0.18, rule_y),
        fill=(*RHUMB_RED, 220),
        width=line,
    )


def draw_frame(
    draw: ImageDraw.ImageDraw, width: int, height: int, factor: float
) -> None:
    """Graduated double frame (alternating ink and parchment segments)."""
    outer, inner = 14 * factor, 30 * factor
    w, h = width * factor, height * factor
    line = max(1, round(factor * 2.0))
    draw.rectangle(
        (outer, outer, w - outer, h - outer), outline=(*INK, 255), width=line * 2
    )
    draw.rectangle(
        (inner, inner, w - inner, h - inner), outline=(*INK, 255), width=line
    )
    band = (outer + inner) / 2.0
    segment = 60 * factor
    for index, start in enumerate(np.arange(inner, w - inner, segment)):
        if index % 2 == 0:
            end = min(start + segment, w - inner)
            draw.rectangle((start, outer + line, end, inner - line), fill=(*INK, 200))
            draw.rectangle(
                (start, h - inner + line, end, h - outer - line), fill=(*INK, 200)
            )
    for index, start in enumerate(np.arange(inner, h - inner, segment)):
        if index % 2 == 0:
            end = min(start + segment, h - inner)
            draw.rectangle((outer + line, start, inner - line, end), fill=(*INK, 200))
            draw.rectangle(
                (w - inner + line, start, w - outer - line, end), fill=(*INK, 200)
            )
    del band


# --- Assembly -------------------------------------------------------------------------


def render(
    width: int = WIDTH,
    height: int = HEIGHT,
    map_dir: Path = MAP_DIR,
    factions_dir: Path = FACTIONS_DIR,
    seed: int = SEED,
) -> tuple[Image.Image, dict]:
    """Render the illustration; returns the RGB image and the layout sidecar."""
    rng = np.random.default_rng(seed)
    meta = _load_json(map_dir / "map.json")
    provinces = _load_json(map_dir / "provinces.geojson")
    map_size = tuple(int(value) for value in meta["size_px"])
    frame = choose_frame(provinces, map_size, width, height)
    metres_per_px = float(meta.get("meters_per_px", 700.0)) / frame.scale
    heights, land = load_rasters(map_dir, frame, width, height)

    # Raster base: parchment, cooler sea, hillshade wash, coast ripples.
    color = parchment(width, height, rng)
    color = _mix(color, SEA, (1.0 - land) * 0.55)
    shade = hillshade(heights, metres_per_px)
    relief = np.clip((heights - 150.0) / 1800.0, 0.0, 1.0)
    wash = land * (1.0 - shade) * (0.18 + 0.30 * relief)
    color = _mix(color, INK, wash * 0.55)
    ripples = coast_ripples(land, (0.42, 0.22, 0.10, 0.04))
    color = _mix(color, INK, ripples * 0.30)

    factor = float(SUPERSAMPLE)
    base = Image.fromarray(np.clip(color, 0, 255).astype(np.uint8), mode="RGB")
    canvas = base.resize(
        (width * SUPERSAMPLE, height * SUPERSAMPLE), Image.Resampling.BICUBIC
    ).convert("RGBA")
    draw = ImageDraw.Draw(canvas, "RGBA")

    cartouche = cartouche_box(width, height)
    rose_radius = height * 0.085
    rose = find_rose_center(land, rose_radius * 1.3, cartouche)
    draw_rhumb_lines(draw, rose, math.hypot(width, height), factor)

    borders = [_project(frame, ring, factor) for _properties, ring in _lines(provinces)]
    _draw_polylines(draw, borders, (*RHUMB_RED, 55), factor * 1.0)

    draw_hachures(draw, heights, land, shade, metres_per_px, factor, rng)

    rivers = _lines(_load_json(map_dir / "rivers.geojson"))
    for properties, line in rivers:
        rank = int(properties.get("scalerank", 10) or 10)
        river_width = factor * (1.9 if rank <= 7 else 1.2)
        _draw_polylines(
            draw, [_project(frame, line, factor)], (*RIVER_INK, 200), river_width
        )

    coast = [
        _project(frame, line, factor)
        for _p, line in _lines(_load_json(map_dir / "coastline.geojson"))
    ]
    _draw_polylines(draw, coast, (*INK, 90), factor * 5.0)
    _draw_polylines(draw, coast, (*INK, 255), factor * 2.2)

    draw_mountains(draw, heights, land, factor, rng)

    city_font = _load_font(round(height * 0.017 * factor), italic=True)
    for name, position in faction_capitals(factions_dir, provinces):
        x, y = frame.to_out(*position)
        if 40 < x < width - 200 and 40 < y < height - 40:
            draw_city(draw, (x, y), name, height * 0.0075, factor, city_font)

    draw_compass_rose(draw, rose, rose_radius, factor)
    draw_cartouche(draw, cartouche, factor)
    draw_frame(draw, width, height, factor)

    image = canvas.resize((width, height), Image.Resampling.LANCZOS).convert("RGB")
    # Final paper grain (after downsampling, to stay crisp).
    grain = rng.normal(0.0, 3.0, (height, width, 1)).astype(np.float32)
    pixels = np.asarray(image, dtype=np.float32) + grain
    image = Image.fromarray(np.clip(pixels, 0, 255).astype(np.uint8), mode="RGB")
    sidecar = {
        "size": [width, height],
        "cartouche": list(cartouche),
        "compass_rose": [round(rose[0]), round(rose[1]), round(rose_radius)],
        "map_window": [round(value, 1) for value in frame.box(width, height)],
    }
    return image, sidecar


def build(
    out_dir: Path = UI_ASSETS_DIR,
    width: int = WIDTH,
    height: int = HEIGHT,
    map_dir: Path = MAP_DIR,
) -> Path:
    """Write ``menu_map.jpg`` and ``menu_map.json`` into ``out_dir``; returns the image path."""
    image, sidecar = render(width=width, height=height, map_dir=map_dir)
    out_dir.mkdir(parents=True, exist_ok=True)
    path = out_dir / OUTPUT_NAME
    image.save(path, quality=JPEG_QUALITY, optimize=True, subsampling=0)
    (out_dir / SIDECAR_NAME).write_text(
        json.dumps(sidecar, indent=2) + "\n", encoding="utf-8"
    )
    return path
