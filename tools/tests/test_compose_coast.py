"""Estuaires, baies et presqu'îles d'auteur de la carte composée (lot mapgeo11)."""

from __future__ import annotations

import json

import jsonschema
import numpy as np
import pytest
from PIL import Image

from cent_ans_tools.compose.compile import (
    add_headlands,
    carve_inlets,
    tapered_shape_mask,
)
from cent_ans_tools.compose.contract import ComposedWarp
from cent_ans_tools.paths import DATA_DIR

MAP_DIR = DATA_DIR / "map"
COMPOSE_DIR = MAP_DIR / "compose"
Image.MAX_IMAGE_PIXELS = None


@pytest.fixture(scope="module")
def warp() -> ComposedWarp:
    """Déformation relue depuis le contrat versionné."""
    return ComposedWarp.load(COMPOSE_DIR)


@pytest.fixture(scope="module")
def config() -> dict:
    """Règles de généralisation d'auteur."""
    return json.loads((COMPOSE_DIR / "generalise.json").read_text(encoding="utf-8"))


def test_generalise_config_matches_its_schema(config):
    """Les estuaires et presqu'îles respectent le schéma (un largeur par sommet)."""
    schema = json.loads(
        (DATA_DIR / "schemas" / "compose_generalise.schema.json").read_text(
            encoding="utf-8"
        )
    )
    jsonschema.validate(config, schema)
    for shape in config["inlets"] + config["headlands"]:
        assert len(shape["path"]) == len(shape["widths_km"]), shape["name"]


def test_tapered_mask_follows_the_width_profile(warp):
    """Large à l'embouchure, étroit en amont ; carve retire, headlands ajoute."""
    shape = (warp.size_px[1], warp.size_px[0])
    item = {"name": "t", "path": [[-1.22, 45.62], [-0.56, 45.04]], "widths_km": [20, 6]}
    mask = tapered_shape_mask(shape, warp, [item])
    ys, xs = np.nonzero(mask)
    assert len(xs) > 500
    first = mask[:, : xs.min() + 40].sum()
    last = mask[:, xs.max() - 40 :].sum()
    assert first > 1.5 * last
    land = np.ones(shape, dtype=bool)
    carve_inlets(land, warp, [item])
    assert not land[mask].any() and land[~mask].all()
    sea = np.zeros(shape, dtype=bool)
    add_headlands(sea, warp, [item])
    assert (sea == mask).all()


def test_gironde_and_mont_saint_michel_bay_are_water():
    """Le masque versionné a l'estuaire de la Gironde et le fond de baie du Mont."""
    land = np.asarray(Image.open(MAP_DIR / "land_mask.png")) > 0
    warp = ComposedWarp.load(COMPOSE_DIR)
    for name, lon, lat in (
        ("Gironde", -0.95, 45.45),
        ("baie du Mont-Saint-Michel", -1.75, 48.73),
    ):
        x, y = warp.forward(np.array([lon]), np.array([lat]))
        assert not land[int(y[0]), int(x[0])], name
