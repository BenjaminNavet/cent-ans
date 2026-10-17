"""Lacs flottants du relief composé remis à leur vrai niveau (lot mapgeo)."""

from __future__ import annotations

import json

import numpy as np
from PIL import Image

from cent_ans_tools.compose import lake_patch
from cent_ans_tools.paths import DATA_DIR

Image.MAX_IMAGE_PIXELS = None


def _square_lake(level: float) -> dict:
    return {
        "level_m": level,
        "polygon_px": [[20.0, 20.0], [40.0, 20.0], [40.0, 40.0], [20.0, 40.0]],
    }


def test_detects_the_real_level_under_a_floating_lake():
    """Un palier plat bas dans l'anneau de rive est le vrai niveau ; un lac sans palier est laissé."""
    heights = np.full((64, 64), 700.0)
    heights[20:40, 20:40] = 607.0
    rules = lake_patch.LakeRules()
    assert lake_patch.detect_plateau(heights, _square_lake(607.0), rules) is None
    heights[41:46, 18:42] = 366.0
    assert lake_patch.detect_plateau(heights, _square_lake(607.0), rules) == 366.0


def test_ignores_a_small_or_sea_level_plateau():
    """Moins de 30 pixels, ou un niveau sous 3 m : pas de correction."""
    heights = np.full((64, 64), 700.0)
    heights[41:42, 18:30] = 366.0
    rules = lake_patch.LakeRules()
    assert lake_patch.detect_plateau(heights, _square_lake(607.0), rules) is None
    heights[41:46, 18:42] = 0.0
    assert lake_patch.detect_plateau(heights, _square_lake(607.0), rules) is None


def test_retouch_replaces_only_the_old_level_inside_the_box():
    """Seuls les pixels à l'ancien niveau dans la boîte du lac changent."""
    metres = np.full((40, 40), 700.0)
    metres[5:10, 5:10] = 607.0
    metres[30:35, 30:35] = 607.0
    item = {"old": 607.0, "new": 366.0, "box": (0, 0, 20, 20)}
    out = lake_patch.retouch(metres, 0, 0, 1, [item], 0.1)
    assert (out[5:10, 5:10] == 366.0).all()
    assert (out[30:35, 30:35] == 607.0).all()
    assert (out[0:5, 0:5] == 700.0).all()


def test_no_lake_of_the_map_floats_above_a_real_lower_level():
    """Après `compose lake-patch`, aucun lac de lakes.json ne laisse un palier réel plus bas (Léman à 366 m)."""
    assert lake_patch.plan(DATA_DIR / "map") == []
    doc = json.loads((DATA_DIR / "map" / "lakes.json").read_text(encoding="utf-8"))
    leman = next(lake for lake in doc["lakes"] if lake["name"] == "Léman")
    assert leman["level_m"] == 366.0
