"""Masques d'auteur de la carte composée (lot CC, C4a) : forêt, roche, champs, marais.

``data/map/compose/masks/{forest,rock,fields,marsh}.png`` (éditeur de carte, format dans
``docs/editeur-carte.md``) : PNG 8 bits, une case = 16 px composés, 0 = absent, 255 = présent. Un
masque jamais peint n'a pas de fichier (lire = rien). Ils surchargent la couverture du sol calculée
(``landcover``) : ils s'ajoutent (``max``), l'effacement au pinceau retire simplement la case du
masque. Effets :

- ``forest`` : forêt présente (le couvert monte à ``m``), prime sur les champs et la roche ;
- ``rock`` : lande / roche (canal A du splat, ``heath``) ;
- ``fields`` : terroir cultivé, retire forêt et zones humides, la part ouverte passe en champs ;
- ``marsh`` : marais (canal R de ``wetlands.png``), retire la forêt.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.paths import DATA_DIR

COMPOSE_DIR = DATA_DIR / "map" / "compose"
MASK_CELL_PX = 16
MASK_NAMES = ("forest", "rock", "fields", "marsh")


def load_mask(
    name: str, shape: tuple[int, int], compose_dir: Path = COMPOSE_DIR
) -> np.ndarray | None:
    """Masque ``name`` en ``float32`` [0, 1] à la taille ``shape`` (rows, cols) du canevas, ou ``None``.

    Les cases de 16 px sont agrandies avec une interpolation bilinéaire (bords doux) ; un masque
    entièrement vide donne ``None``.
    """
    path = compose_dir / "masks" / f"{name}.png"
    if not path.exists():
        return None
    with Image.open(path) as image:
        cells = np.asarray(image.convert("L"), dtype=np.float32) / 255.0
    if not cells.any():
        return None
    rows, cols = shape
    # Centre d'une case = (i + 0.5) * 16 px : coordonnées de cases pour chaque pixel.
    ys = (np.arange(rows, dtype=np.float32) + 0.5) / MASK_CELL_PX - 0.5
    xs = (np.arange(cols, dtype=np.float32) + 0.5) / MASK_CELL_PX - 0.5
    grid_y, grid_x = np.meshgrid(ys, xs, indexing="ij")
    out = ndimage.map_coordinates(cells, [grid_y, grid_x], order=1, mode="nearest")
    return np.clip(out, 0.0, 1.0).astype(np.float32)


def load_all(
    shape: tuple[int, int], compose_dir: Path = COMPOSE_DIR
) -> dict[str, np.ndarray]:
    """Les masques non vides, par nom."""
    found = {}
    for name in MASK_NAMES:
        mask = load_mask(name, shape, compose_dir)
        if mask is not None:
            found[name] = mask
    return found


def apply_wetlands(wet: np.ndarray, masks: dict[str, np.ndarray]) -> np.ndarray:
    """``wetlands`` (rows, cols, 3) avec les marais peints (R) et sans les zones de champs."""
    if "marsh" in masks:
        wet[..., 0] = np.maximum(wet[..., 0], masks["marsh"])
    if "fields" in masks:
        wet *= (1.0 - masks["fields"])[..., None]
    return wet


def apply_cover(
    forest: np.ndarray, heath: np.ndarray, masks: dict[str, np.ndarray]
) -> tuple[np.ndarray, np.ndarray]:
    """Couvert forestier et lande après surcharge (forêt ajoutée, retirée par champs / marais / roche)."""
    if "fields" in masks:
        forest = forest * (1.0 - masks["fields"])
        heath = heath * (1.0 - masks["fields"])
    if "marsh" in masks:
        forest = forest * (1.0 - masks["marsh"])
    if "rock" in masks:
        heath = np.maximum(heath, masks["rock"])
        forest = forest * (1.0 - masks["rock"])
    if "forest" in masks:
        forest = np.maximum(forest, masks["forest"])
        heath = heath * (1.0 - masks["forest"])
    return forest.astype(np.float32), heath.astype(np.float32)


def apply_splat(weights: np.ndarray, masks: dict[str, np.ndarray]) -> np.ndarray:
    """Poids du splat (R herbe, G cultures, B forêt, A roche) : les champs peints prennent la part ouverte."""
    if "fields" in masks:
        field = masks["fields"][..., None]
        land = weights.sum(axis=-1, keepdims=True) > 0.5
        target = np.zeros_like(weights)
        target[..., 1] = 1.0
        mixed = weights * (1.0 - field) + target * field
        weights = np.where(land, mixed, weights)
    return weights
