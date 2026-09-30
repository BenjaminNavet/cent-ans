"""Campaign biome map (chantier HB, ADR 0143): ``cent-ans geo biomes``.

Skeleton: see ``data/map/biomes.yaml`` for the legend and thresholds.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

from cent_ans_tools.geo import download

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
LEGEND_PATH = MAP_DIR / "biomes.yaml"
SCHEMA_PATH = REPO_DIR / "data" / "schemas" / "biomes.schema.json"
BIOMES_NAME = "biomes.png"


def load_legend(path: Path = LEGEND_PATH, schema_path: Path = SCHEMA_PATH) -> dict:
    """Parse ``biomes.yaml`` and validate it against its schema."""
    raise NotImplementedError


def classify(*args, **kwargs) -> np.ndarray:
    """Biome index of every pixel (uint8)."""
    raise NotImplementedError


def build(map_dir: Path = MAP_DIR, legend_path: Path = LEGEND_PATH) -> list[Path]:
    """``geo biomes``: bake ``biomes.png``."""
    raise NotImplementedError
