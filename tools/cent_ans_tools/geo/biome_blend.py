"""Biome border blend maps for the campaign ground (TX T2b3, ADR 0243): ``cent-ans geo biome-blend``.

Bakes two half-resolution L8 rasters from ``data/map/biomes.png`` (sea filled with the nearest
land biome, so coasts never blend):

* ``biomes_blend_ab.png``: ``A + 16 * B`` where A is the pixel's biome and B the nearest other
  biome (1..14, nibbles);
* ``biomes_blend_dist.png``: distance (half-resolution texels, capped at 255) from the pixel to
  the nearest pixel of another biome. Linearly filtered by the shader, which turns it into the
  weight of B (0.5 on the border, 0 beyond the blend width) so both sides of a border agree.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage

from cent_ans_tools.geo.biomes import MAP_DIR, read_biomes

AB_NAME = "biomes_blend_ab.png"
DIST_NAME = "biomes_blend_dist.png"
STEP = 2  # half resolution


def blend_maps(biomes: np.ndarray, step: int = STEP) -> tuple[np.ndarray, np.ndarray]:
    """(A + 16 B, distance) rasters from a biome index map (0 = sea)."""
    sub = np.ascontiguousarray(biomes[::step, ::step])
    land = sub > 0
    if not land.all():
        indices = ndimage.distance_transform_edt(
            ~land, return_distances=False, return_indices=True
        )
        sub = sub[indices[0], indices[1]]
    present = [int(v) for v in np.unique(sub)]
    best = np.full(sub.shape, np.inf, dtype=np.float32)
    second = np.full(sub.shape, np.inf, dtype=np.float32)
    best_id = sub.copy()
    second_id = sub.copy()
    for biome in present:
        distance = ndimage.distance_transform_edt(sub != biome).astype(np.float32)
        closer = distance < best
        between = ~closer & (distance < second)
        second = np.where(closer, best, np.where(between, distance, second))
        second_id = np.where(closer, best_id, np.where(between, biome, second_id))
        best = np.where(closer, distance, best)
        best_id = np.where(closer, biome, best_id)
    second_id = np.where(np.isfinite(second), second_id, sub)
    packed = (sub.astype(np.uint8)) | (second_id.astype(np.uint8) << 4)
    dist = np.clip(np.where(np.isfinite(second), second, 255.0), 0, 255).astype(np.uint8)
    return packed, dist


def build(map_dir: Path = MAP_DIR) -> list[Path]:
    """Write both rasters next to ``biomes.png``."""
    biomes = read_biomes(map_dir)
    if biomes is None:
        raise FileNotFoundError("biomes.png absent : lancer `cent-ans geo biomes`")
    packed, dist = blend_maps(biomes)
    paths = []
    for name, array in ((AB_NAME, packed), (DIST_NAME, dist)):
        path = map_dir / name
        Image.fromarray(array, mode="L").save(path, optimize=True)
        paths.append(path)
    return paths
