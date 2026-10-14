"""Provinces de la carte composée (lot CC, C4b).

Une province composée est l'union des géométries RÉELLES des provinces d'origine qu'elle absorbe
(``data/map/compose/id_map.json``), passées par la déformation. On ne refait donc pas de Voronoï :
pour chaque pixel composé, la déformation inverse donne le pixel réel, dont l'ancienne étiquette
(``province_ids.png`` d'avant composition, ``data/map/real/``) est convertie en nouvelle province.
La côte est celle de ``land_mask.png`` (composé) ; les pixels de terre sans étiquette proches d'une
province (côte généralisée, lacs comblés) en prennent l'étiquette, les grandes terres sans province
restent à 0 (Sahara, steppe) ; les enclaves de quelques pixels sont absorbées.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.compose.warp import COMPOSE_DIR, real_path

#: Une terre sans étiquette plus petite que ça (pixels composés) prend l'étiquette voisine.
FILL_MAX_AREA_PX = 4000
#: Une composante d'une province plus petite que ça est absorbée par sa voisine (ou supprimée).
MIN_COMPONENT_PX = 60
CHUNK_ROWS = 512


def _decode(rgb: np.ndarray) -> np.ndarray:
    return rgb[..., 0].astype(np.uint16) | (rgb[..., 1].astype(np.uint16) << 8)


def real_labels() -> tuple[np.ndarray, dict[int, str]]:
    """Étiquettes de l'ancienne carte (grille réelle) et ``index -> id`` d'origine."""
    with Image.open(real_path("province_ids.png")) as image:
        labels = _decode(np.asarray(image.convert("RGB")))
    collection = json.loads(real_path("provinces.geojson").read_text("utf-8"))
    index_to_id = {
        f["properties"]["index"]: f["properties"]["id"] for f in collection["features"]
    }
    return labels, index_to_id


def pull_labels(
    cw: ComposedWarp, new_index: dict[str, int], id_map: dict[str, str | None]
) -> np.ndarray:
    """Étiquette composée brute de chaque pixel par déformation inverse (0 = aucune)."""
    old, index_to_id = real_labels()
    lut = np.zeros(max(index_to_id) + 1, dtype=np.uint16)
    for old_index, old_id in index_to_id.items():
        target = id_map.get(old_id)
        if target is not None:
            lut[old_index] = new_index[target]
    width, height = cw.size_px
    out = np.zeros((height, width), dtype=np.uint16)
    xs = np.arange(width, dtype=np.float64) + 0.5
    for r0 in range(0, height, CHUNK_ROWS):
        r1 = min(r0 + CHUNK_ROWS, height)
        gx, gy = np.meshgrid(xs, np.arange(r0, r1, dtype=np.float64) + 0.5)
        rx, ry = cw.inverse_px(gx, gy)
        ok = np.isfinite(rx) & np.isfinite(ry)
        ix = np.clip(
            np.floor(np.where(ok, rx, 0)).astype(np.int64), 0, old.shape[1] - 1
        )
        iy = np.clip(
            np.floor(np.where(ok, ry, 0)).astype(np.int64), 0, old.shape[0] - 1
        )
        out[r0:r1] = np.where(ok, lut[old[iy, ix]], 0)
    return out


def _nearest_fill(labels: np.ndarray, where: np.ndarray) -> np.ndarray:
    """Remplit ``where`` par l'étiquette du pixel étiqueté le plus proche."""
    _, indices = ndimage.distance_transform_edt(labels == 0, return_indices=True)
    return np.where(where, labels[indices[0], indices[1]], labels)


def fill_small_land_gaps(
    labels: np.ndarray, land: np.ndarray
) -> tuple[np.ndarray, int]:
    """Terre sans étiquette : les composantes < :data:`FILL_MAX_AREA_PX` prennent la voisine."""
    gaps = land & (labels == 0)
    comp, n = ndimage.label(gaps)
    if n == 0:
        return labels, 0
    areas = np.bincount(comp.ravel())
    small = (areas[comp] < FILL_MAX_AREA_PX) & gaps
    return _nearest_fill(labels, small), int(small.sum())


def absorb_small_components(labels: np.ndarray) -> tuple[np.ndarray, int]:
    """Composantes (4-connexes) de province < :data:`MIN_COMPONENT_PX` : absorbées ou supprimées."""
    out = labels.copy()
    changed = 0
    present = np.unique(labels)
    for value in present[present > 0]:
        mask = labels == value
        comp, n = ndimage.label(mask)
        if n <= 1:
            continue
        areas = np.bincount(comp.ravel())
        for c in np.nonzero(areas[1:] < MIN_COMPONENT_PX)[0] + 1:
            part = comp == c
            ring = ndimage.binary_dilation(part) & ~part
            around = labels[ring]
            around = around[(around > 0) & (around != value)]
            out[part] = np.bincount(around).argmax() if around.size else 0
            changed += int(part.sum())
    return out, changed


def build_labels(
    cw: ComposedWarp, land: np.ndarray, provinces_dir: Path, smooth
) -> tuple[np.ndarray, dict]:
    """Étiquettes composées finales, ``(labels, rapport)``."""
    ids = sorted(p.stem for p in provinces_dir.glob("*.json"))
    new_index = {pid: i for i, pid in enumerate(ids, start=1)}
    id_map = json.loads((COMPOSE_DIR / "id_map.json").read_text("utf-8"))["provinces"]
    raw = pull_labels(cw, new_index, id_map)
    labels = np.where(land, raw, 0).astype(np.uint16)
    labels, filled = fill_small_land_gaps(labels, land)
    labels = smooth(labels, labels > 0, 5)
    labels, absorbed = absorb_small_components(labels)
    labels, refilled = fill_small_land_gaps(labels, land)
    present = set(np.unique(labels).tolist()) - {0}
    missing = [pid for pid, i in new_index.items() if i not in present]
    report = {
        "pull_land_px": int((raw[land] > 0).sum()),
        "land_px": int(land.sum()),
        "filled_px": filled + refilled,
        "absorbed_px": absorbed,
        "unlabelled_land_px": int((land & (labels == 0)).sum()),
        "missing_provinces": missing,
    }
    return labels, report
