"""Fleuves de vallée (Nil, Euphrate, Tigre) ajoutés à rivers.geojson (lot CC CS2)."""

from __future__ import annotations

import json

import numpy as np

from cent_ans_tools.compose import valley_rivers
from cent_ans_tools.geo import colormap
from cent_ans_tools.geo.provinces import load_grid


def test_reach_sea_cuts_at_the_coast():
    """Un tracé qui entre en mer est coupé à la première case de mer."""
    land = np.zeros((20, 20), dtype=bool)
    land[:, :10] = True
    points = np.array([[2.0, 5.0], [8.0, 5.0], [14.0, 5.0], [18.0, 5.0]])
    out = valley_rivers.reach_sea(points, land)
    assert out[-1][0] == 14.0
    assert len(out) == 3


def test_reach_sea_extends_a_river_that_stops_inland():
    """Un tracé qui s'arrête avant la côte est prolongé jusqu'à elle."""
    land = np.zeros((20, 20), dtype=bool)
    land[:, :12] = True
    points = np.array([[2.0, 5.0], [6.0, 5.0]])
    out = valley_rivers.reach_sea(points, land)
    assert out[-1][0] > 11.0


def test_valley_features_are_dense_named_rivers():
    """Nil (et ses bras), Euphrate et Tigre : tracés denses et nommés."""
    features = valley_rivers.build_features()
    names = {f["name"] for f in features}
    assert {"Nile", "Nile (Rosetta branch)", "Nile (Damietta branch)"} <= names
    assert "Euphrates" in names
    nile = next(f for f in features if f["name"] == "Nile")
    assert len(nile["coords"]) > 100  # tracé dense, pas trois points
    assert all(f["source"] == "valley" for f in features)


def test_rivers_geojson_carries_the_valley_rivers():
    """rivers.geojson porte les fleuves de vallée et garde ceux du compilateur."""
    document = json.loads(
        (valley_rivers.MAP_DIR / "rivers.geojson").read_text(encoding="utf-8")
    )
    sources = {
        f["properties"].get("name"): f["properties"].get("source")
        for f in document["features"]
    }
    assert sources.get("Nile") == "valley"
    assert sources.get("Loire") == "composed"  # les 45 fleuves du compilateur restent


def test_irrigated_areas_fade_instead_of_a_flat_straight_edge():
    """Delta : pseudo-distance pleine au coeur, nulle irrigation hors du fondu, valeurs intermédiaires au bord."""
    grid = load_grid(valley_rivers.MAP_DIR)
    document = json.loads(
        (valley_rivers.MAP_DIR / valley_rivers.COURSES_FILE).read_text(encoding="utf-8")
    )
    features = colormap._course_features(document, grid)
    assert features["features"], "delta attendu"
    ring = np.array(features["features"][0]["geometry"]["coordinates"][0])
    centre = ring.mean(axis=0)
    shape = (int(ring[:, 1].max()) + 200, int(ring[:, 0].max()) + 200)
    river_km = np.full(shape, 24.0, dtype=np.float32)
    colormap.irrigated_into(river_km, features, grid, 9.0, 0.65)
    assert river_km[int(centre[1]), int(centre[0])] <= 0.65 * 9.0  # bande pleine
    assert (
        river_km[int(ring[:, 1].min()) - 120, int(centre[0])] == 24.0
    )  # loin : intact
    edge = river_km[int(ring[:, 1].min()) - 20 : int(ring[:, 1].max()), int(centre[0])]
    assert (
        (edge > 0.65 * 9.0) & (edge < 9.0)
    ).sum() > 10  # bord fondu sur de nombreuses cases
