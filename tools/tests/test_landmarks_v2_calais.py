"""Calais around 1340 (lot VH8): dated key facts of data/landmarks_v2/calais.json."""

import json
from pathlib import Path

import pytest

from cent_ans_tools.geo import landmarks_v2

DATA = Path(__file__).resolve().parents[2] / "data"


def _city() -> dict:
    return json.loads(
        (DATA / "landmarks_v2" / "calais.json").read_text(encoding="utf-8")
    )


def _present(item: dict, year: int) -> bool:
    return item.get("from_year", -9999) <= year <= item.get("until_year", 9999)


def test_calais_walls_1228() -> None:
    """Hurepel's wall of 1228, four gates, double wall and ditch found by the English in 1346."""
    city = _city()
    walls = {w["id"]: w for w in city["walls"]}
    main = walls["enceinte"]
    assert main["from_year"] == 1228 and main["closed"]
    gates = {g["name"] for g in main["gates"]}
    assert gates == {
        "Porte du Havre",
        "Porte de Boulogne",
        "Porte de Gravelines",
        "Porte vers Saint-Pierre",
    }
    xs = [q[0] for q in main["points"]]
    assert max(xs) - min(xs) == pytest.approx(1100, abs=120)  # 1 100 m (Réseau Vauban)
    length = landmarks_v2.wall_length({"walls": [main]})
    assert 2800.0 < length < 3500.0, length
    assert main["tower_spacing_m"] * 44 == pytest.approx(length, rel=0.02)  # 44 tours
    assert main["thickness_m"] == pytest.approx(2.5)  # boulevard du 8-Mai, 2017
    assert _present(walls["avant_mur"], 1340)  # two walls, two ditches


def test_calais_1340_monuments() -> None:
    """French town in 1340: no English works yet; Notre-Dame before its Perpendicular rebuild."""
    city = _city()
    monuments = {m["id"]: m for m in city["monuments"]}
    present = {i for i, m in monuments.items() if _present(m, 1340)}
    assert {
        "tour_du_guet",
        "chateau",
        "notre_dame_xiiie",
        "saint_nicolas",
        "hotel_de_ville",
    } <= present
    assert not present & {
        "etape_des_laines",
        "rysbank_fortin",
        "tour_de_rysbank",
        "notre_dame_anglaise",
        "notre_dame_tour",
    }
    # Castle at the north-west corner of the town (Greaves 1918, SRA 2025).
    castle = monuments["chateau"]["at"]
    others = [m["at"] for i, m in monuments.items() if i != "chateau"]
    assert castle[0] < min(q[0] for q in others if q[1] < 300)
    # Rysbank: English fort of 1347 on the spit, stone tower later; Staple from 1363.
    assert monuments["rysbank_fortin"]["from_year"] == 1347
    assert (
        monuments["rysbank_fortin"]["until_year"] + 1
        == monuments["tour_de_rysbank"]["from_year"]
    )
    assert monuments["etape_des_laines"]["from_year"] == 1363
    assert monuments["etape_des_laines"]["certainty"] == "hypothetical"
    # Notre-Dame: XIIIth-c. church with two façade towers, English church, then crossing tower.
    old = monuments["notre_dame_xiiie"]
    assert len(old["params"]["west_towers"]) == 2
    assert old["until_year"] + 1 == monuments["notre_dame_anglaise"]["from_year"]
    assert "crossing" not in monuments["notre_dame_anglaise"]["params"]
    tower = monuments["notre_dame_tour"]["params"]["crossing"]
    assert tower["height"] + tower["spire_m"] == pytest.approx(56, abs=2)
    assert (
        monuments["notre_dame_tour"]["params"]["length_m"] < 90
    )  # 1631 chapel excluded


def test_calais_waters_and_streets() -> None:
    """No fine river: the 1340 harbour is a hand polygon; OSM streets without the later boulevards."""
    city = _city()
    assert city["fine_rivers"] == []
    havre = {w["id"]: w for w in city["waters"]}["havre"]
    assert havre["origin"] == "hand" and "polygon" in havre
    osm = {s["name"] for s in city["streets"] if s["origin"] == "osm"}
    assert {"Rue Royale", "Rue de la Mer", "Rue André Gerschel"} <= osm
    assert not osm & {
        "Boulevard des Alliés",
        "Boulevard de la Résistance",
        "Rue de Moscou",
    }
    assert len([s for s in city["streets"] if s["origin"] == "osm"]) > 40
