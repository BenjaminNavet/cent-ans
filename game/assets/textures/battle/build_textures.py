"""Construit les textures de bataille (lot V4, 2k GA2) depuis Poly Haven (CC0) et par procédure.

Usage : uv run --with pillow --with numpy python build_textures.py <dossier_telechargements>

Le dossier doit contenir, pour les couches du sol listées dans
`data/fx/battle_ground_layers.json` (lot GA2, schéma `fx_battle_ground_layers.schema.json`),
les fichiers Poly Haven `<id>_diff_2k.jpg` (albédo, 2k) et `<id>_nor_gl_1k.jpg` (normale, 1k —
mesure mémoire GA2 : 13 couches en 2k/2k dépasseraient 120 Mo, 2k/1k tient à ~87 Mo) ; pour les
couches uniques (bâtiments, écorce), `<id>_diff_1k.jpg` et `<id>_nor_gl_1k.jpg`
(https://api.polyhaven.com/files/<id>). Produit, à côté de ce script :
- `ground_albedo_array.jpg` (2048 px/couche) / `ground_normal_array.jpg` (1024 px/couche) :
  couches du sol empilées verticalement (importées en Texture2DArray), ordre et identifiants
  Poly Haven tirés de `data/fx/battle_ground_layers.json` (jamais codés en dur ici) ;
- `<id>_diff.jpg` / `<id>_nor.jpg` : textures des murailles, toits, maisons, écorce ;
- `foliage_leaves.png`, `grass_clump.png` : cartes alpha procédurales (feuillage, touffes d'herbe).
"""

import json
import math
import random
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw, ImageFilter

HERE = Path(__file__).parent
DATA_ROOT = HERE.parents[3] / "data"

GROUND_LAYERS = [
    layer["poly_haven_id"]
    for layer in json.loads(
        (DATA_ROOT / "fx" / "battle_ground_layers.json").read_text(encoding="utf-8")
    )["layers"]
]

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


def stack(src: Path, suffix: str, source_res: str, size: int, out: str) -> None:
    """Empile les couches du sol (ordre de `battle_ground_layers.json`) verticalement."""
    sheet = Image.new("RGB", (size, size * len(GROUND_LAYERS)))
    for i, layer in enumerate(GROUND_LAYERS):
        image = Image.open(src / f"{layer}_{suffix}_{source_res}.jpg").convert("RGB")
        sheet.paste(image.resize((size, size), Image.LANCZOS), (0, i * size))
    sheet.save(HERE / out, quality=88)


def singles(src: Path) -> None:
    """Copie les textures des bâtiments et de l'écorce (normales réduites à 512)."""
    for name in SINGLE:
        diff = Image.open(src / f"{name}_diff_1k.jpg").convert("RGB")
        diff.save(HERE / f"{name}_diff.jpg", quality=88)
        nor = Image.open(src / f"{name}_nor_gl_1k.jpg").convert("RGB")
        nor.resize((512, 512), Image.LANCZOS).save(HERE / f"{name}_nor.jpg", quality=90)


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
    # GA2 : albédo 2k, normale 1k (mesure mémoire, cf. docs/wip/ga.md section GA2).
    stack(downloads, "diff", "2k", 2048, "ground_albedo_array.jpg")
    stack(downloads, "nor_gl", "1k", 1024, "ground_normal_array.jpg")
    singles(downloads)
    leaves()
    grass()
