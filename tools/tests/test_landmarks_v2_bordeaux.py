"""Bordeaux around 1340 (VH8): key dated facts of data/landmarks_v2/bordeaux.json."""

import json
from pathlib import Path

import pytest

from cent_ans_tools.geo import landmarks_v2

CITY = Path(__file__).resolve().parents[2] / "data" / "landmarks_v2" / "bordeaux.json"


@pytest.fixture(scope="module")
def city() -> dict:
    """The Bordeaux v2 file."""
    return json.loads(CITY.read_text(encoding="utf-8"))


def test_links(city: dict) -> None:
    """Linked to the v1 maquette and the settlement; Garonne from the fine map, no bridge."""
    assert city["landmark"] == "bordeaux"
    assert city["settlement"] == "set_bordeaux"
    assert city["fine_rivers"] == ["Garonne"]
    assert any(w["origin"] == "rivers_fine" for w in city["waters"])
    # Pas de pont sur la Garonne avant le pont de pierre (1822).
    assert not city.get("bridges")


def test_enceintes_1340(city: dict) -> None:
    """Third enceinte (1302-1327) in place; castrum and second enceinte kept as inner walls."""
    walls = {w["id"]: w for w in city["walls"]}
    land, river = walls["enceinte_front_terre"], walls["enceinte_front_garonne"]
    assert not any("from_year" in w or "until_year" in w for w in walls.values())
    # Fossés en eau du côté de la terre seulement (Drouyn).
    assert land["ditch_m"] > 0 and river["ditch_m"] == 0
    assert land["points"][-1] == river["points"][0]
    assert land["points"][0] == river["points"][-1]
    gates = {g["name"] for w in (land, river) for g in w["gates"]}
    assert {"Porte Saint-Germain", "Porte Dijeaux", "Porte Saint-Julien"} <= gates
    assert "Porte du Caillau" in gates
    assert "Porte Cailhau" not in gates  # porte actuelle : 1493-1496
    # Castrum : 725 x 450 m (Drouyn), 740 x 480 m (Wikipédia).
    xs = [p[0] for p in walls["castrum"]["points"]]
    ys = [p[1] for p in walls["castrum"]["points"]]
    assert 650 < max(xs) - min(xs) < 760
    assert 430 < max(ys) - min(ys) < 500
    second = {g["name"] for g in walls["deuxieme_enceinte"]["gates"]}
    assert "Porte Saint-Éloi (Saint-James)" in second
    total = landmarks_v2.wall_length({"walls": [land, river]})
    assert 5000.0 < total < 7500.0, total


def test_monuments_dated(city: dict) -> None:
    """Pey-Berland from 1440, Saint-Michel rebuilt (hall church 1430), no French castles."""
    monuments = {m["id"]: m for m in city["monuments"]}
    assert monuments["pey_berland"]["from_year"] == 1440
    assert monuments["saint_michel_ancienne"]["until_year"] + 1 == 1430
    assert monuments["saint_michel"]["from_year"] == 1430
    # 124 m dans œuvre (Wikipédia), ≈ 129 m hors œuvre (OSM).
    assert monuments["cathedrale"]["params"]["length_m"] == pytest.approx(126, abs=5)
    assert "west_towers" not in monuments["cathedrale"]["params"]
    # Château Trompette (1453-1455) et fort du Hâ (1454-1456) : après la période.
    assert not {"chateau_trompette", "fort_du_ha"} & set(monuments)
    for m in city["monuments"]:
        assert m.get("certainty") in {"attested", "probable", "hypothetical"}, m["id"]
        assert m.get("description"), m["id"]


def test_streets(city: dict) -> None:
    """Medieval streets from OSM, XVIIIe-XIXe-s. cours excluded."""
    osm = [s for s in city["streets"] if s["origin"] == "osm"]
    assert len(osm) > 200
    names = {s["name"] for s in osm}
    assert "Rue Sainte-Catherine" in names
    assert not {"Cours de l'Intendance", "Allées de Tourny", "Cours Pasteur"} & names
