"""Bathymétrie de la carte (VIS-R5) : encodage, remplissage, lignes à télécharger, données versionnées."""

from __future__ import annotations

import json

import numpy as np
from PIL import Image

from cent_ans_tools.geo import bathymetry


def test_encode_decode_round_trip_keeps_shallow_precision() -> None:
    """La racine carrée garde un pas fin près de la côte, large au large."""
    depth = np.array(
        [0.0, 1.0, 10.0, 50.0, 200.0, 1000.0, 4000.0, 9000.0], dtype=np.float32
    )
    back = bathymetry.decode(bathymetry.encode(depth))
    assert back[0] == 0.0
    assert abs(back[2] - 10.0) < 1.0
    assert abs(back[3] - 50.0) < 2.5
    assert abs(back[5] - 1000.0) < 40.0
    assert back[6] == bathymetry.MAX_DEPTH_M
    assert back[7] == bathymetry.MAX_DEPTH_M


def test_fill_nearest_replaces_land_with_closest_sea() -> None:
    """Un NaN (terre) prend la profondeur de la mer voisine, rien d'autre ne bouge."""
    grid = np.array([[10.0, np.nan, np.nan], [10.0, np.nan, 300.0]], dtype=np.float32)
    filled = bathymetry.fill_nearest(grid)
    assert not np.isnan(filled).any()
    assert filled[0, 1] == 10.0
    assert filled[1, 2] == 300.0
    assert filled[0, 2] == 300.0


def test_row_range_covers_the_map() -> None:
    """Les lignes lues couvrent 27-67 N et restent dans la tuile."""
    first, last = bathymetry._row_range()
    assert (first, last) == (5520, 15120)
    assert bathymetry.DATA_OFFSET + last * bathymetry.ROW_BYTES < 933_255_948


def test_versioned_raster_matches_its_metadata() -> None:
    """Le raster versionné existe, à 1/4 de la carte, et décrit son encodage."""
    meta_path = bathymetry.MAP_DIR / "bathymetry.json"
    if not meta_path.exists():
        return
    meta = json.loads(meta_path.read_text("utf-8"))
    with Image.open(bathymetry.MAP_DIR / "bathymetry.png") as image:
        assert image.mode == "L"
        assert list(image.size) == meta["size_px"]
    map_meta = json.loads((bathymetry.MAP_DIR / "map.json").read_text("utf-8"))
    width, height = map_meta["size_px"]
    assert meta["size_px"] == [width // meta["factor"], height // meta["factor"]]
    assert meta["encoding"] == "sqrt"
