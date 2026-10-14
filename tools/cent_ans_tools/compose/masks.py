"""Masques d'auteur de la carte composée (lot CC, C4a) : forêt, roche, champs, marais.

``data/map/compose/masks/{forest,rock,fields,marsh}.png`` (éditeur de carte, format dans
``docs/editeur-carte.md``) : PNG 8 bits, une case = 16 px composés, 0 = absent, 255 = présent. Un
masque jamais peint n'a pas de fichier (lire = rien). Ils surchargent la couverture du sol calculée
(``landcover``) : ils s'ajoutent (``max``), l'effacement au pinceau retire simplement la case du
masque. Effets :

- ``forest`` : forêt présente (la valeur peinte fixe la part boisée, en massifs : ``massif_cover``), prime sur les champs et la roche ;
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


def apply_wetlands(
    wet: np.ndarray, masks: dict[str, np.ndarray], noise: np.ndarray | None = None
) -> np.ndarray:
    """``wetlands`` (rows, cols, 3) : marais retirés, puis marais peints (R), sans les zones de champs.

    Avec ``noise`` (``massif_noise(..., MARSH_SCALES, MARSH_SEED)``), la valeur peinte fixe la PART
    de marais et la forme vient du bruit (nappes et roselières, pas un aplat lisse).
    """
    if "marsh_clear" in masks:
        wet[..., 0] *= 1.0 - masks["marsh_clear"]
    if "marsh" in masks:
        painted = (
            masks["marsh"] if noise is None else massif_cover(masks["marsh"], noise)
        )
        wet[..., 0] = np.maximum(wet[..., 0], painted)
    if "fields" in masks:
        wet *= (1.0 - masks["fields"])[..., None]
    return wet


MASSIF_EDGE = 0.02  # demi-largeur (en rang de bruit) du bord d'un massif peint
MASSIF_SEED = 361  # ADR 0361 : graine du bruit à massifs de la forêt peinte
MASSIF_SCALES = (26, 70, 170)  # mêmes échelles que ``compute_forest`` (``noise_mix``)
MASSIF_WEIGHTS = (0.5, 0.3, 0.2)
# CP4 : champs et marais peints, même principe que la forêt, échelles plus fines, graines distinctes.
FIELD_SEED = 362
FIELD_SCALES = (60, 160, 400)  # cases sur le grand côté : parcellaires de ~20 à ~120 px
MARSH_SEED = 363
MARSH_SCALES = (90, 240, 600)  # nappes et roselières de ~10 à ~80 px


def massif_noise(
    shape: tuple[int, int],
    weights: tuple[float, ...] = MASSIF_WEIGHTS,
    seed: int = MASSIF_SEED,
    scales: tuple[int, ...] = MASSIF_SCALES,
) -> np.ndarray:
    """Bruit à massifs de la forêt peinte : rang uniforme [0, 1], plusieurs échelles (comme ``compute_forest``)."""
    from cent_ans_tools.geo import landcover

    rng = np.random.default_rng(seed)
    noise = np.zeros(shape, dtype=np.float32)
    for weight, cells in zip(weights, scales, strict=False):
        noise += np.float32(weight) * landcover.uniform_noise(
            shape, rng, base_cells=cells, octaves=3
        )
    order = np.argsort(noise, axis=None, kind="stable")
    ranks = np.empty(noise.size, dtype=np.float32)
    ranks[order] = np.linspace(0.0, 1.0, noise.size, dtype=np.float32)
    return ranks.reshape(shape)


def massif_cover(amount: np.ndarray, noise: np.ndarray) -> np.ndarray:
    """Couvert en massifs irréguliers : la valeur peinte ``amount`` fixe la PART boisée visée.

    Le bruit est un rang uniforme : ``noise > seuil(v)`` couvre une fraction ``v`` de la surface
    (seuil = ``1 + e - v (1 + 2e)``, donc 0 pour v = 0 et plein pour v = 1). Le bord du trait
    (``v`` qui décroît) se fond par ce seuil : les massifs se raréfient, sans dégradé de densité.
    """
    edge = MASSIF_EDGE
    threshold = (1.0 + edge) - amount * (1.0 + 2.0 * edge)
    t = np.clip((noise - (threshold - edge)) / (2.0 * edge), 0.0, 1.0)
    cover = t * t * (3.0 - 2.0 * t)
    return np.where(amount > 1e-3, cover, 0.0).astype(np.float32)


def apply_cover(
    forest: np.ndarray,
    heath: np.ndarray,
    masks: dict[str, np.ndarray],
    noise: np.ndarray | None = None,
) -> tuple[np.ndarray, np.ndarray]:
    """Couvert forestier et lande après surcharge (retraits, puis forêt ajoutée, retirée par champs / marais / roche).

    Avec ``noise`` (``massif_noise``), la forêt peinte est découpée en massifs et clairières par seuil
    de bruit (``massif_cover``) ; sans lui, ``max(couvert, v)`` lisse (ancien comportement).
    """
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
        painted = (
            masks["forest"] if noise is None else massif_cover(masks["forest"], noise)
        )
        forest = np.maximum(forest, painted)
        heath = heath * (1.0 - painted)
    return forest.astype(np.float32), heath.astype(np.float32)


def apply_splat(
    weights: np.ndarray,
    masks: dict[str, np.ndarray],
    noise: np.ndarray | None = None,
) -> np.ndarray:
    """Poids du splat (R herbe, G cultures, B forêt, A roche) : retraits (rendus à l'herbe), puis les champs peints prennent la part ouverte.

    Avec ``noise`` (``massif_noise(..., FIELD_SCALES, FIELD_SEED)``, mêmes lignes que ``weights``),
    la valeur peinte fixe la PART cultivée, découpée en parcelles par le bruit.
    """
    for key, channel in SPLAT_CLEAR_CHANNELS.items():
        if key in masks:
            moved = weights[..., channel] * masks[key]
            weights = weights.copy()
            weights[..., channel] -= moved
            weights[..., 0] += moved
    if "fields" in masks:
        painted = (
            masks["fields"] if noise is None else massif_cover(masks["fields"], noise)
        )
        field = painted[..., None]
        land = weights.sum(axis=-1, keepdims=True) > 0.5
        target = np.zeros_like(weights)
        target[..., 1] = 1.0
        mixed = weights * (1.0 - field) + target * field
        weights = np.where(land, mixed, weights)
    return weights
