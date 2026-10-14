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

Retrait (lot CP) : ``<nom>_clear.png`` (même format) retire du calculé : une valeur ``v`` multiplie la
couverture calculée de la couche par ``1 - v`` (forêt : cover et splat B ; roche : lande et splat A ;
champs : splat G ; marais : canal R des zones humides). Le retrait passe avant l'ajout. Chaque nom a
deux sources, prises au ``max`` : le pinceau (``masks/<nom>.png``, éditeur) et la peinture vectorielle
rastérisée (``masks/derived/<nom>.png``, ``cent-ans compose paint``). Fichier absent = aucun effet.
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
CLEAR_SUFFIX = "_clear"
SPLAT_CLEAR_CHANNELS = {"fields_clear": 1, "forest_clear": 2, "rock_clear": 3}


def _read_cells(path: Path) -> np.ndarray | None:
    if not path.exists():
        return None
    with Image.open(path) as image:
        return np.asarray(image.convert("L"), dtype=np.float32) / 255.0


def load_mask(
    name: str, shape: tuple[int, int], compose_dir: Path = COMPOSE_DIR
) -> np.ndarray | None:
    """Masque ``name`` (ou ``<nom>_clear``) en ``float32`` [0, 1] à la taille ``shape`` (rows, cols), ou ``None``.

    Max du pinceau (``masks/``) et du dérivé vectoriel (``masks/derived/``). Les cases de 16 px sont
    agrandies avec une interpolation bilinéaire (bords doux) ; un masque entièrement vide donne ``None``.
    """
    cells = None
    for directory in (compose_dir / "masks", compose_dir / "masks" / "derived"):
        part = _read_cells(directory / f"{name}.png")
        if part is None:
            continue
        if cells is None:
            cells = part
        else:
            rows, cols = (
                min(a, b) for a, b in zip(cells.shape, part.shape, strict=True)
            )
            cells = cells.copy()
            cells[:rows, :cols] = np.maximum(cells[:rows, :cols], part[:rows, :cols])
    if cells is None or not cells.any():
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
    """Les masques non vides, par nom (``forest``… et ``forest_clear``…)."""
    found = {}
    for name in MASK_NAMES:
        for key in (name, f"{name}{CLEAR_SUFFIX}"):
            mask = load_mask(key, shape, compose_dir)
            if mask is not None:
                found[key] = mask
    derived = load_erosion_rock(shape, compose_dir)
    if derived is not None:
        found["rock"] = (
            derived if "rock" not in found else np.maximum(found["rock"], derived)
        )
    return found


def load_erosion_rock(
    shape: tuple[int, int], compose_dir: Path = COMPOSE_DIR
) -> np.ndarray | None:
    """Roche et éboulis dérivés de l'érosion (lot CD, ``geo compose-erode``), ou ``None``.

    ``derived/erosion_masks.png`` : R roche (pente forte, lits de ravines), G éboulis (pied des
    parois), B neige (non consommée ici : le shader dérive la neige de l'altitude et de la pente).
    L'éboulis compte pour une roche à moitié (pierrier clair, peu de forêt).
    """
    path = compose_dir / "derived" / "erosion_masks.png"
    if not path.exists():
        return None
    with Image.open(path) as image:
        rgb = np.asarray(image.convert("RGB"), dtype=np.float32) / 255.0
    rock = np.maximum(rgb[..., 0], 0.5 * rgb[..., 1])
    rows, cols = shape
    out = np.zeros(shape, dtype=np.float32)
    out[: min(rows, rock.shape[0]), : min(cols, rock.shape[1])] = rock[:rows, :cols]
    return out if out.any() else None


def apply_wetlands(wet: np.ndarray, masks: dict[str, np.ndarray]) -> np.ndarray:
    """``wetlands`` (rows, cols, 3) : marais retirés, puis marais peints (R), sans les zones de champs."""
    if "marsh_clear" in masks:
        wet[..., 0] *= 1.0 - masks["marsh_clear"]
    if "marsh" in masks:
        wet[..., 0] = np.maximum(wet[..., 0], masks["marsh"])
    if "fields" in masks:
        wet *= (1.0 - masks["fields"])[..., None]
    return wet


def apply_cover(
    forest: np.ndarray, heath: np.ndarray, masks: dict[str, np.ndarray]
) -> tuple[np.ndarray, np.ndarray]:
    """Couvert forestier et lande après surcharge (retraits, puis forêt ajoutée, retirée par champs / marais / roche)."""
    if "forest_clear" in masks:
        forest = forest * (1.0 - masks["forest_clear"])
    if "rock_clear" in masks:
        heath = heath * (1.0 - masks["rock_clear"])
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
    """Poids du splat (R herbe, G cultures, B forêt, A roche) : retraits (rendus à l'herbe), puis les champs peints prennent la part ouverte."""
    for key, channel in SPLAT_CLEAR_CHANNELS.items():
        if key in masks:
            moved = weights[..., channel] * masks[key]
            weights = weights.copy()
            weights[..., channel] -= moved
            weights[..., 0] += moved
    if "fields" in masks:
        field = masks["fields"][..., None]
        land = weights.sum(axis=-1, keepdims=True) > 0.5
        target = np.zeros_like(weights)
        target[..., 1] = 1.0
        mixed = weights * (1.0 - field) + target * field
        weights = np.where(land, mixed, weights)
    return weights
