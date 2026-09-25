"""Relief pyramid of the campaign map, tiers 1-2 (chantier ZG, ADR 0036).

Quadtree of 512² 16-bit tiles in ``data/map/pyramid/E{level}/{col}_{row}.png``
(gitignored), same encoding and EPSG:3035 grid as the E0 tiles of
``data/map/height/``: level ``k`` covers ``256 / 2**k`` world units per tile,
``360 / 2**k`` m per pixel.

- E1-E2 (tier 1): Copernicus GLO-90 area average, all land of the fine bbox.
- E3-E4 (tier 2): Copernicus GLO-30, surface model corrected (canopy under ESA
  WorldCover trees, modern built-up flattened, post-1340 reservoirs filled),
  on the core bbox only.

Every level carries the render boost of ``heightmap_render.png`` (ADR 0019),
computed once on the finest source then averaged, so that a 2 x 2 mean of
children gives back their parent. Writes ``data/map/relief_pyramid.json``.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

PYRAMID_DIR_NAME = "pyramid"
TILE_PX = 512
ROOT_TILE_UNITS = 256


@dataclass(frozen=True)
class TileKey:
    """Address of one pyramid tile."""

    level: int
    col: int
    row: int


@dataclass
class PyramidResult:
    """Summary of a pyramid build."""

    tiles_written: int
    total_bytes: int
    seconds: float


def tile_path(map_dir: Path, key: TileKey) -> Path:
    """Path of a tile of level >= 1 in the gitignored cache."""
    return map_dir / PYRAMID_DIR_NAME / f"E{key.level}" / f"{key.col}_{key.row}.png"


def build(levels: tuple[int, ...] = (1, 2, 3, 4), force: bool = False) -> PyramidResult:
    """Bake the requested levels and rewrite the manifest (lot ZG1)."""
    raise NotImplementedError("lot ZG1")
