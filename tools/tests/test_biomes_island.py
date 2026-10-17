"""Île de Bretagne océanique (lot england) : Londres et le sud-est ne sont plus « continental »."""

import json

import numpy as np
from PIL import Image

from cent_ans_tools.geo import biomes

MAP_DIR = biomes.MAP_DIR


def _biome_at(name: str) -> int:
    settlements = json.loads((MAP_DIR / "settlements_px.json").read_text())
    x, y = (int(v) for v in settlements[name][:2])
    return int(np.asarray(Image.open(MAP_DIR / "biomes.png"))[y, x])


def test_legend_with_island_boxes_validates():
    legend = biomes.load_legend()
    assert legend["rules"]["oceanic_land_boxes"]


def test_london_and_kent_are_oceanic_paris_stays_continental():
    ids = biomes.indices(biomes.load_legend())
    assert _biome_at("set_londres") == ids["oceanic"]
    assert _biome_at("set_cantorbery") == ids["oceanic"]
    assert _biome_at("set_paris") == ids["continental"]
    assert _biome_at("set_rouen") == ids["continental"]
