"""Construit l'atlas des cartes de feuillage des arbres proches de la carte (lot FC5, CC0).

Usage : uv run --with pillow --with numpy python build_leaf_cards.py

Produit `campaign_leaf_cards.png` (1024 × 512) à côté de ce script : moitié gauche = rameau
feuillu des feuillus (`../battle/leaf_spray.png`, procédural CC0, lot DA6), moitié droite =
brindille de sapin (Poly Haven `fir_tree_01`, CC0). Chaque moitié est divisée par sa couleur
moyenne (linéaire) puis multipliée par 0,5 : la texture ne module que la palette des sommets
(`VegetationMeshes.PALETTES`), le shader (`foliage.gdshaderinc`, FOLIAGE_CARDS) multiplie par 2.
Couleurs des pixels transparents diffusées depuis les bords (pas de franges au mipmapping).

Produit aussi `campaign_grass_tuft.png` : copie de `../battle/grass_blades.png` (lot DA6, CC0),
importée ici avec mipmaps pour les touffes d'herbe de la carte (`GroundClutter`, lot FC5).
"""

from pathlib import Path

import numpy as np
from PIL import Image

HERE = Path(__file__).parent
REPO = HERE.parents[3]
SOURCES = [
    REPO / "game/assets/textures/battle/leaf_spray.png",
    REPO / "game/assets/third_party/vegetation/fir_tree_01/fir_tree_01_fir_tree_01_twig_diff_alpha.png",
]
SIDE = 512


def srgb_to_linear(c: np.ndarray) -> np.ndarray:
    return np.where(c <= 0.04045, c / 12.92, ((c + 0.055) / 1.055) ** 2.4)


def linear_to_srgb(c: np.ndarray) -> np.ndarray:
    c = np.clip(c, 0.0, 1.0)
    return np.where(c <= 0.0031308, c * 12.92, 1.055 * c ** (1 / 2.4) - 0.055)


def bleed(rgb: np.ndarray, known: np.ndarray, iterations: int = 32) -> np.ndarray:
    rgb = rgb.copy()
    for _ in range(iterations):
        if known.all():
            break
        acc = np.zeros_like(rgb)
        cnt = np.zeros(known.shape, dtype=np.float32)
        for dy, dx in ((1, 0), (-1, 0), (0, 1), (0, -1)):
            k = np.roll(np.roll(known, dy, 0), dx, 1)
            acc += np.roll(np.roll(rgb, dy, 0), dx, 1) * k[..., None]
            cnt += k
        grow = (~known) & (cnt > 0)
        rgb[grow] = acc[grow] / cnt[grow][:, None]
        known = known | grow
    rgb[~known] = rgb[known].mean(axis=0)
    return rgb


def main() -> None:
    halves = []
    for path in SOURCES:
        image = Image.open(path).convert("RGBA").resize((SIDE, SIDE), Image.LANCZOS)
        arr = np.asarray(image).astype(np.float32) / 255.0
        lin = srgb_to_linear(arr[..., :3])
        opaque = arr[..., 3] > 0.5
        mean = lin[opaque].mean(axis=0)
        normalized = bleed(lin / mean * 0.5, opaque)
        out = np.concatenate([linear_to_srgb(normalized), arr[..., 3:4]], axis=-1)
        halves.append(out)
        print(f"{path.name}: mean linear {mean.round(3)}, coverage {opaque.mean():.2f}")
    atlas = np.concatenate(halves, axis=1)
    Image.fromarray(np.round(atlas * 255).astype(np.uint8), "RGBA").save(HERE / "campaign_leaf_cards.png")
    print("campaign_leaf_cards.png", atlas.shape[1], atlas.shape[0])
    tuft = Image.open(REPO / "game/assets/textures/battle/grass_blades.png").convert("RGBA")
    arr = np.asarray(tuft).astype(np.float32) / 255.0
    arr[..., :3] = bleed(arr[..., :3], arr[..., 3] > 0.5)
    Image.fromarray(np.round(arr * 255).astype(np.uint8), "RGBA").save(HERE / "campaign_grass_tuft.png")
    print("campaign_grass_tuft.png", tuft.size)


if __name__ == "__main__":
    main()
