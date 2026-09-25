"""Construit les cartes alpha procédurales de la végétation de bataille du lot DA6 (CC0).

Usage : uv run --with pillow --with numpy python build_da6_textures.py

Produit, à côté de ce script :
- `grass_blades.png` : touffe d'herbe en éventail (pied resserré, brins fins, quelques épis) ;
  seule la luminance et l'alpha servent, la couleur vient du sol (`battle_grass.gdshader`) ;
- `leaf_spray.png` : rameau feuillu (feuilles de feuillu sur brindilles, bord déchiqueté, jours) ;
- `twig_spray.png` : ramilles nues d'hiver (feuillus sans feuilles) ;
- `dead_leaves.png` : feuilles sèches clairsemées (chêne marcescent l'hiver).

Couleurs sourdes (bible DA § 3.3 : décor ≤ 35 % de saturation en moyenne une fois éclairé).
"""

import math
import random
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = Path(__file__).parent


def _bleed(image: Image.Image) -> Image.Image:
    """Donne aux pixels transparents la couleur moyenne (pas de franges au mipmapping)."""
    arr = np.array(image).astype(np.float32)
    opaque = arr[:, :, 3] > 0
    arr[~opaque, :3] = arr[opaque][:, :3].mean(axis=0)
    return Image.fromarray(arr.clip(0, 255).astype(np.uint8))


def grass_blades(width: int = 512, height: int = 256) -> None:
    """Touffe en éventail : brins effilés partant d'un pied étroit, épis clairs, pied sombre."""
    rng = random.Random(606)
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    blades = []
    for _ in range(300):
        # Pied resserré (un tiers de la largeur), brins qui s'écartent en montant.
        base_x = width / 2 + rng.gauss(0.0, width / 11)
        h = rng.uniform(0.35, 1.0) * height * 0.97
        spread = (base_x - width / 2) * rng.uniform(0.7, 1.6)
        lean = spread + rng.uniform(-0.18, 0.18) * h
        w = rng.uniform(2.0, 4.6)
        tone = rng.uniform(0.62, 1.2)
        blades.append((h, base_x, lean, w, tone))
    # Les brins courts devant (dessinés en dernier), les longs derrière.
    blades.sort(key=lambda b: -b[0])
    for h, base_x, lean, w, tone in blades:
        straw = rng.random() < 0.2
        v = int(np.clip(120 * tone, 40, 230))
        col = (v, v, v) if not straw else (int(v * 1.12), int(v * 1.08), int(v * 0.9))
        left = []
        right = []
        for s in range(13):
            t = s / 12
            x = base_x + lean * t * t
            y = height - t * h
            half = w * (1.0 - t) ** 0.8 * 0.5
            left.append((x - half, y))
            right.append((x + half, y))
        draw.polygon(left + right[::-1], fill=(*col, 255))
        if straw and h > height * 0.6 and rng.random() < 0.6:
            # Épi (graminées en fleur) : petits grains au sommet.
            tx = base_x + lean
            ty = height - h
            for k in range(9):
                gy = ty + k * 2.2
                gx = tx - lean * (k * 2.2 / h) * 1.8 + rng.uniform(-1.5, 1.5)
                g = int(np.clip(v * 1.25, 60, 245))
                draw.ellipse((gx - 1.6, gy - 2.4, gx + 1.6, gy + 2.4), fill=(g, g, int(g * 0.9), 255))
    arr = np.array(image).astype(np.float32)
    rows = np.linspace(0.0, 1.0, height)
    # Pied assombri (occlusion dans la touffe), pointes un peu plus claires.
    ramp = (1.0 - (rows**2.2) * 0.55) * (1.0 + (1.0 - rows) ** 3 * 0.12)
    arr[:, :, :3] *= ramp[:, None, None]
    _bleed(Image.fromarray(arr.clip(0, 255).astype(np.uint8))).save(HERE / "grass_blades.png")


def _leaf(draw: ImageDraw.ImageDraw, x: float, y: float, size: float, angle: float, col: tuple) -> None:
    """Feuille ovale pointue (feuillus d'Europe), côté éclairé légèrement plus clair."""
    pts = []
    for k in range(12):
        t = k / 12 * math.tau
        px = math.cos(t) * size
        # Pointe : le demi-côté avant s'affine.
        py = math.sin(t) * size * 0.42 * (1.0 - 0.35 * max(math.cos(t), 0.0))
        pts.append((x + px * math.cos(angle) - py * math.sin(angle), y + px * math.sin(angle) + py * math.cos(angle)))
    draw.polygon(pts, fill=col)


def leaf_spray(size: int = 512) -> None:
    """Rameau feuillu : brindilles ramifiées portant des feuilles, masse irrégulière avec jours."""
    rng = random.Random(2025)
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    twigs = []

    def grow(x: float, y: float, angle: float, length: float, depth: int) -> None:
        steps = 6
        for s in range(steps):
            nx = x + math.cos(angle) * length / steps
            ny = y + math.sin(angle) * length / steps
            draw.line((x, y, nx, ny), fill=(62, 52, 42, 255), width=max(1, depth + 1))
            twigs.append((nx, ny, depth))
            x, y = nx, ny
            angle += rng.uniform(-0.18, 0.18)
            if depth > 0 and s in (2, 4) and rng.random() < 0.9:
                side = rng.choice((-1, 1))
                grow(x, y, angle + side * rng.uniform(0.5, 0.95), length * rng.uniform(0.45, 0.62), depth - 1)

    for k in range(3):
        grow(size * 0.5 + rng.uniform(-30, 30), size * 0.98, -math.pi / 2 + rng.uniform(-0.55, 0.55) + (k - 1) * 0.35, size * 0.62, 2)
    c = size / 2
    for _ in range(5200):
        tx, ty, depth = rng.choice(twigs)
        a = rng.random() * math.tau
        d = rng.uniform(2.0, 44.0) * math.sqrt(rng.random())
        x = tx + math.cos(a) * d
        y = ty + math.sin(a) * d
        r = math.hypot(x - c, y - c * 0.9) / (c * 0.95)
        if r > 1.0 or not (8 < x < size - 8 and 8 < y < size - 8):
            continue
        # Cœur plus sombre (ombre propre), bord et haut plus clairs.
        shade = 0.6 + 0.4 * r + (c - y) / size * 0.25 + rng.uniform(-0.14, 0.14)
        g = float(np.clip(112 * shade, 36, 190))
        red = g * rng.uniform(0.7, 0.86)
        blue = g * rng.uniform(0.42, 0.55)
        _leaf(draw, x, y, rng.uniform(7.0, 12.0), rng.random() * math.tau, (int(red), int(g), int(blue), 255))
    _bleed(image.filter(ImageFilter.SMOOTH)).save(HERE / "leaf_spray.png")


def twig_spray(size: int = 512) -> None:
    """Ramilles nues : branches ramifiées fines, gris brun (feuillus en hiver)."""
    rng = random.Random(77)
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)

    def grow(x: float, y: float, angle: float, length: float, width: float, depth: int) -> None:
        steps = 7
        for s in range(steps):
            nx = x + math.cos(angle) * length / steps
            ny = y + math.sin(angle) * length / steps
            tone = rng.uniform(0.85, 1.1)
            draw.line((x, y, nx, ny), fill=(int(78 * tone), int(70 * tone), int(62 * tone), 255), width=max(1, round(width)))
            x, y = nx, ny
            angle += rng.uniform(-0.22, 0.22)
            width *= 0.93
            if depth > 0 and s % 2 == 1:
                side = rng.choice((-1, 1))
                grow(x, y, angle + side * rng.uniform(0.35, 0.8), length * rng.uniform(0.4, 0.6), width * 0.7, depth - 1)

    for k in range(4):
        grow(size * 0.5 + rng.uniform(-40, 40), size * 0.99, -math.pi / 2 + (k - 1.5) * 0.3 + rng.uniform(-0.2, 0.2), size * 0.7, 4.0, 3)
    _bleed(image).save(HERE / "twig_spray.png")


def dead_leaves(size: int = 512) -> None:
    """Feuilles sèches clairsemées accrochées aux ramilles (chênes marcescents l'hiver)."""
    rng = random.Random(1340)
    base = Image.open(HERE / "twig_spray.png").convert("RGBA")
    arr = np.array(base)
    image = base.copy()
    draw = ImageDraw.Draw(image)
    ys, xs = np.nonzero(arr[:, :, 3] > 0)
    for _ in range(380):
        i = rng.randrange(len(xs))
        x = xs[i] + rng.uniform(-5, 5)
        y = ys[i] + rng.uniform(-3, 6)
        v = rng.uniform(0.75, 1.15)
        _leaf(draw, x, y, rng.uniform(6.0, 10.0), rng.random() * math.tau, (int(112 * v), int(94 * v), int(74 * v), 255))
    _bleed(image).save(HERE / "dead_leaves.png")


if __name__ == "__main__":
    grass_blades()
    leaf_spray()
    twig_spray()
    dead_leaves()
