"""Campaign ground colour map, "satellite" style (chantier SS, ADR 0141).

Skeleton (lot SS1): public API only.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from pathlib import Path

import numpy as np

from cent_ans_tools.geo import download

REPO_DIR = download.TOOLS_DIR.parent
MAP_DIR = REPO_DIR / "data" / "map"
STYLE_PATH = MAP_DIR / "colormap_style.yaml"
SCHEMA_PATH = REPO_DIR / "data" / "schemas" / "colormap_style.schema.json"
STEM = "colormap_bc1"
MAP_KEY = "colormap.bc1"
PREVIEW_NAME = "colormap_preview.jpg"


@dataclass
class ColormapInputs:
    """Rasters (map grid) and vectors (map pixels) the colour map is painted from."""

    land: np.ndarray
    splat: np.ndarray
    height_m: np.ndarray
    coast_dist_px: np.ndarray
    wetlands: np.ndarray
    conifer: np.ndarray
    lon: np.ndarray
    lat: np.ndarray
    meters_per_px: float
    roads: list[tuple[str, np.ndarray]] = field(default_factory=list)
    settlements: np.ndarray = field(default_factory=lambda: np.zeros((0, 2)))
    towns: list[tuple[float, float, float]] = field(default_factory=list)
    reservoirs: np.ndarray = field(default_factory=lambda: np.zeros((0, 2)))


def load_style(path: Path = STYLE_PATH) -> dict:
    """Parse and validate ``colormap_style.yaml``."""
    raise NotImplementedError


def load_inputs(map_dir: Path = MAP_DIR) -> ColormapInputs:
    """Read the rasters and vectors of ``map_dir``."""
    raise NotImplementedError


def bake(inputs: ColormapInputs, style: dict, band_rows: int = 1024) -> np.ndarray:
    """RGB colour map (``scale`` x the map grid), uint8."""
    raise NotImplementedError


def build(map_dir: Path = MAP_DIR, style_path: Path = STYLE_PATH) -> list[Path]:
    """``geo colormap``: bake, BC1 parts with mipmaps, ``map.json`` entry, preview."""
    raise NotImplementedError
