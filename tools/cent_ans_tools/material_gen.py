"""GA: AI-generated tileable materials and their derived maps.

Pipeline: prompt (``data/art/materials.yaml``) -> 1024x1024 image via
:func:`cent_ans_tools.openrouter.generate_image` (budget-guarded) -> tileable
(half offset + seam blend) -> derived maps (height from high-passed luminance,
OpenGL normal, roughness) -> contact sheet for review.
"""

from __future__ import annotations

from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[2]
MATERIALS_PATH = ROOT / "data" / "art" / "materials.yaml"


def generate(material_id: str, out_dir: Path) -> Path:
    """Generate the raw image of one material and return its path."""
    raise NotImplementedError


def make_tileable(image: np.ndarray, blend_width: int) -> np.ndarray:
    """Return a seamlessly tileable copy of ``image`` (H x W x C, uint8)."""
    raise NotImplementedError


def derive_maps(albedo: np.ndarray) -> dict[str, np.ndarray]:
    """Derive ``height``, ``normal`` (OpenGL) and ``roughness`` maps from an albedo."""
    raise NotImplementedError


def contact_sheet(tiles: dict[str, np.ndarray], out_path: Path) -> Path:
    """Write a labelled review sheet of ``tiles`` and return its path."""
    raise NotImplementedError
