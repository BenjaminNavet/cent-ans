"""TX (ADR 0236): local texture factory, catalogue-driven, for every texture family.

Chain per family (``data/art/textures/<family>.yaml``)::

    generate -> seamless -> upscale -> pbr -> (alpha) -> checks -> pack -> board

Each stage reads the previous one's output on disk, so an interrupted batch resumes.
Raw and intermediate images live outside the repository in ``RAW_ROOT/<family>/``.
"""

from __future__ import annotations

from pathlib import Path

RAW_ROOT = Path.home() / "dev" / "cent-ans-raw" / "textures"
CATALOG_DIR = Path(__file__).resolve().parents[3] / "data" / "art" / "textures"
FAMILIES = (
    "ground_campaign",
    "ground_battle",
    "vegetation_cards",
    "foliage_bark",
    "building_materials",
    "water_surfaces",
    "micro_detail",
)
