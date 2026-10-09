"""Construit les textures de bataille (murailles, toits, écorce, feuillage) depuis Poly Haven (CC0) et par procédure.

Usage : uv run --with pillow --with numpy python build_textures.py <dossier_telechargements>

Le dossier doit contenir, pour les couches uniques (bâtiments, écorce), `<id>_diff_1k.jpg` et
`<id>_nor_gl_1k.jpg` (2k / 1k pour les matières GA5), voir https://api.polyhaven.com/files/<id>.
Le sol de bataille n'est plus produit ici (ADR 0244 : paquets TX, `data/art/tx_battle_*_pack.json`).
Produit, à côté de ce script :
- `<id>_diff.jpg` / `<id>_nor.jpg` : textures des murailles, toits, maisons, écorce ;
- `foliage_leaves.png`, `grass_clump.png` : cartes alpha procédurales (feuillage, touffes d'herbe).
"""

import math
import random
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = Path(__file__).parent

SINGLE = [
    "castle_wall_varriation",
    "roof_slates_02",
    "clay_roof_tiles_02",
    "thatch_roof_angled",
    "plastered_wall_02",
    "cobblestone_floor_01",
    "wood_planks",
    "bark_brown_02",
]

# Lot GA5 : matières de bâtiments partagées avec `buildings/build_textures.py`
# (`BuildingMaterials.SPECS` — Masonry, RoofSlate, Thatch) — albédo 2k, normale 1k (même
# convention que GA2), contrairement aux autres `SINGLE` (écorce, sol, restées en 1k/512,
# hors périmètre GA5).
GA5_BUILDING_SINGLE = {"castle_wall_varriation", "roof_slates_02", "thatch_roof_angled"}


def singles(src: Path) -> None:
    """Copie les textures des bâtiments et de l'écorce.

    Lot GA5 : `GA5_BUILDING_SINGLE` (matières de bâtiments de bataille, `BuildingMaterials.SPECS`)
    passe en albédo 2k / normale 1k ; les autres `SINGLE` (écorce, sol) restent en 1k / 512
    (hors périmètre GA5).
    """
    for name in SINGLE:
        if name in GA5_BUILDING_SINGLE:
            diff = Image.open(src / f"{name}_diff_2k.jpg").convert("RGB")
            diff.save(HERE / f"{name}_diff.jpg", quality=88)
            nor = Image.open(src / f"{name}_nor_gl_1k.jpg").convert("RGB")
            nor.save(HERE / f"{name}_nor.jpg", quality=90)
            continue
        diff = Image.open(src / f"{name}_diff_1k.jpg").convert("RGB")
        diff.save(HERE / f"{name}_diff.jpg", quality=88)
        nor = Image.open(src / f"{name}_nor_gl_1k.jpg").convert("RGB")
        nor.resize((512, 512), Image.Resampling.LANCZOS).save(
            HERE / f"{name}_nor.jpg", quality=90
        )


def _bleed(image: Image.Image) -> Image.Image:
    """Donne aux pixels transparents la couleur moyenne (pas de franges au mipmapping)."""
    arr = np.array(image).astype(np.float32)
    opaque = arr[:, :, 3] > 0
    arr[~opaque, :3] = arr[opaque][:, :3].mean(axis=0)
    return Image.fromarray(arr.clip(0, 255).astype(np.uint8))


def leaves(size: int = 512) -> None:
    """Carte de feuillage : amas de petites feuilles elliptiques, bord irrégulier, alpha."""
    rng = random.Random(1337)
    image = Image.new("RGBA", (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    c = size / 2
    # Houppier en grappes : quelques sous-amas aléatoires dans le disque.
    blobs = [(c, c, c * 0.62)]
    for _ in range(9):
        a = rng.random() * math.tau
        d = rng.uniform(0.35, 0.6) * c
        blobs.append(
            (c + math.cos(a) * d, c + math.sin(a) * d, rng.uniform(0.22, 0.38) * c)
        )
    for _ in range(5200):
        bx, by, br = rng.choices(blobs, weights=[b[2] ** 2 for b in blobs])[0]
        angle = rng.random() * math.tau
        rr = math.sqrt(rng.random()) * br
        x = bx + math.cos(angle) * rr
        y = by + math.sin(angle) * rr
        r = min(math.hypot(x - c, y - c), c * 0.93)
        leaf = rng.uniform(5.0, 10.0)
        a = rng.random() * math.tau
        # Feuilles du cœur plus sombres (ombre propre), bords plus clairs.
        shade = 0.55 + 0.45 * (r / (c * 0.93)) + rng.uniform(-0.15, 0.15)
        g = int(np.clip(118 * shade + rng.uniform(-14, 14), 30, 200))
        red = int(np.clip(g * rng.uniform(0.52, 0.72), 10, 170))
        b = int(np.clip(g * rng.uniform(0.22, 0.36), 5, 90))
        pts = []
        for k in range(8):
            t = k / 8 * math.tau
            px = math.cos(t) * leaf
            py = math.sin(t) * leaf * 0.45
            pts.append(
                (
                    x + px * math.cos(a) - py * math.sin(a),
                    y + px * math.sin(a) + py * math.cos(a),
                )
            )
        draw.polygon(pts, fill=(red, g, b, 255))
    _bleed(image.filter(ImageFilter.SMOOTH)).save(HERE / "foliage_leaves.png")


def grass(width: int = 512, height: int = 256) -> None:
    """Carte de touffe d'herbe : brins effilés partant du bas, teintes variées, alpha."""
    rng = random.Random(42)
    image = Image.new("RGBA", (width, height), (0, 0, 0, 0))
    draw = ImageDraw.Draw(image)
    for _ in range(170):
        base_x = rng.gauss(width / 2, width / 6)
        h = rng.uniform(0.45, 1.0) * height * 0.97
        lean = rng.uniform(-0.35, 0.35) * h
        w = rng.uniform(3.0, 7.0)
        tone = rng.uniform(0.7, 1.15)
        if rng.random() < 0.18:
            col = (int(150 * tone), int(140 * tone), int(70 * tone))
        else:
            col = (int(78 * tone), int(118 * tone), int(40 * tone))
        left = []
        right = []
        for s in range(11):
            t = s / 10
            x = base_x + lean * t * t
            y = height - t * h
            half = w * (1.0 - t) * 0.5
            left.append((x - half, y))
            right.append((x + half, y))
        draw.polygon(left + right[::-1], fill=(*col, 255))
    arr = np.array(image).astype(np.float32)
    # Pied de la touffe assombri (occlusion).
    rows = np.linspace(0.0, 1.0, height)
    ramp = 1.0 - (rows**3) * 0.45
    arr[:, :, :3] *= ramp[:, None, None]
    _bleed(Image.fromarray(arr.clip(0, 255).astype(np.uint8))).save(
        HERE / "grass_clump.png"
    )


if __name__ == "__main__":
    downloads = Path(sys.argv[1])
    singles(downloads)
    leaves()
    grass()
