"""Regional agricultural landscapes (lot ME8): ``agri_regions.png``.

``data/map/agri_landscapes.json`` lists landscapes (open fields, bocage, terraced vines,
olive groves, huertas, swidden) and their regions as lon/lat ellipses (same shapes as
``wetlands.json``). This step bakes them into an L8 image, a quarter of the map grid
(one texel = 4 x 4 map pixels): 0 = no override (the biome decides), ``FIRST_ROW + i`` =
landscape ``i`` in the file order (a later region wins where regions overlap). The game
turns each landscape into a row of the ground table (``HbGround``). Rendering only.
"""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.geo import download, landcover
from cent_ans_tools.geo.project import grid_from_metadata

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
LANDSCAPES_FILE = "agri_landscapes.json"
OUTPUT_FILE = "agri_regions.png"
#: First ground-table row of a landscape (rows 1-7 are the biomes).
FIRST_ROW = 8
FACTOR = 4
SEED = 1342
#: Texel where the mask is set when the area mask is at least this strong.
THRESHOLD = 0.5


def landscape_rows(document: dict) -> dict[str, int]:
    """Landscape id -> ground-table row, in file order."""
    return {
        name: FIRST_ROW + index for index, name in enumerate(document["landscapes"])
    }


def bake_index(document: dict, grid, land: np.ndarray) -> np.ndarray:
    """Full-resolution uint8 raster of landscape rows (0 where no region applies)."""
    rows = landscape_rows(document)
    rng = np.random.default_rng(SEED)
    noise = landcover.uniform_noise(grid.shape, rng, base_cells=220, octaves=4)
    out = np.zeros(grid.shape, dtype=np.uint8)
    for region in document["regions"]:
        mask, window = landcover.area_mask(region, grid, noise)
        view = out[window]
        view[mask >= THRESHOLD] = rows[region["landscape"]]
    out[~land] = 0
    return out


def build(map_dir: Path = MAP_DIR) -> Path:
    """Write ``agri_regions.png`` next to the other map rasters."""
    from cent_ans_tools.geo import terrain  # noqa: PLC0415

    document = json.loads((map_dir / LANDSCAPES_FILE).read_text(encoding="utf-8"))
    grid = grid_from_metadata(
        json.loads((map_dir / "map.json").read_text(encoding="utf-8"))
    )
    height = terrain.uint16_to_height(terrain.read_png16(map_dir / "heightmap.png"))
    with Image.open(map_dir / "land_mask.png") as image:
        land = (np.asarray(image.convert("L")) > 127) & (height > 0.0)
    index = bake_index(document, grid, land)
    small = index[FACTOR // 2 :: FACTOR, FACTOR // 2 :: FACTOR]
    path = map_dir / OUTPUT_FILE
    Image.fromarray(np.ascontiguousarray(small), mode="L").save(path, compress_level=9)
    return path
