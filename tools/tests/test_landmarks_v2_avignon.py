"""Avignon around 1340 (lot VH8): dated key facts of ``data/landmarks_v2/avignon.json``."""

import json
import math
from pathlib import Path

import pytest

from cent_ans_tools.geo import landmarks_v2

CITY = Path(__file__).resolve().parents[2] / "data" / "landmarks_v2" / "avignon.json"


@pytest.fixture(scope="module")
def city() -> dict:
    """The Avignon v2 file."""
    return json.loads(CITY.read_text(encoding="utf-8"))


def _alive(item: dict, year: int) -> bool:
    return item.get("from_year", -9999) <= year <= item.get("until_year", 9999)


def test_walls_1340(city: dict) -> None:
    """XIIIth-c. wall in 1340; the ramparts of Innocent VI and Urban V only from 1357."""
    walls = {w["id"]: w for w in city["walls"]}
    old, new = walls["enceinte_xiiie"], walls["remparts"]
    assert _alive(old, 1340) and not _alive(new, 1340)
    assert new["from_year"] == 1357 and new["closed"]
    # Street-named portails of the 1234-1248 wall; the seven late XIVth-c. gates.
    assert {
        "Portail Matheron",
        "Portail Peint (Imbert vieux)",
        "Portail Magnanen",
        "Portail Boquier",
        "Porte Évêque",
        "Portail Bienson",
    } <= {g["name"] for g in old["gates"]}
    assert {g["name"] for g in new["gates"]} == {
        "Porte du Rhône",
        "Porte de l'Oulle",
        "Porte Saint-Roch",
        "Porte Saint-Michel",
        "Porte Limbert",
        "Porte Saint-Lazare",
        "Porte de la Ligne",
    }
    ramparts_m = landmarks_v2.wall_length({"walls": [new]})
    assert 4000.0 < ramparts_m < 5000.0, ramparts_m  # 4 330 m (Wikipédia)
    assert landmarks_v2.wall_length({"walls": [old]}) < ramparts_m


def test_palace_phases(city: dict) -> None:
    """Palais Vieux (1335) standing in 1340, Palais Neuf from 1342, towers dated."""
    monuments = {m["id"]: m for m in city["monuments"]}
    assert monuments["palais_vieux"]["from_year"] == 1335
    assert monuments["palais_neuf"]["from_year"] == 1342
    assert monuments["tour_campane"]["from_year"] == 1340
    assert monuments["tour_trouillas"]["from_year"] == 1346
    assert monuments["tour_trouillas"]["params"]["height"] == 52
    assert monuments["tour_des_anges"]["params"]["height"] == 46
    alive = {i for i, m in monuments.items() if _alive(m, 1340)}
    assert {"palais_vieux", "tour_des_anges", "tour_campane"} <= alive
    assert not alive & {
        "palais_neuf",
        "tour_trouillas",
        "tour_garde_robe",
        "tour_saint_laurent",
    }


def test_dated_churches_and_forts(city: dict) -> None:
    """Rebuilt or later foundations are dated (Saint-Didier 1356, Célestins 1395, fort 1362)."""
    monuments = {m["id"]: m for m in city["monuments"]}
    assert monuments["saint_didier_ancienne"]["until_year"] + 1 == 1356
    assert monuments["saint_didier"]["from_year"] == 1356
    assert monuments["celestins"]["from_year"] == 1395
    assert monuments["saint_martial"]["from_year"] == 1363
    assert monuments["chartreuse"]["from_year"] == 1356
    assert monuments["fort_saint_andre"]["from_year"] == 1362
    assert (
        monuments["notre_dame_des_doms"]["until_year"] == 1404
    )  # bell tower fell in 1405
    assert monuments["tour_philippe_le_bel_1360"]["from_year"] == 1360
    for m in monuments.values():
        assert m["certainty"] in {"attested", "probable", "hypothetical"}
        assert m.get("description"), m["id"]


def test_bridge_saint_benezet(city: dict) -> None:
    """22 arches, ≈ 900 m, 4 m deck, from the town to the tour Philippe-le-Bel."""
    bridges = city["bridges"]
    assert sum(b["arches"] for b in bridges) == 22
    length = sum(math.dist(b["from"], b["to"]) for b in bridges)
    assert length == pytest.approx(900, abs=40), length
    assert all(b["width_m"] == pytest.approx(4.0) for b in bridges)
    tower = next(m for m in city["monuments"] if m["id"] == "tour_philippe_le_bel")
    assert math.dist(bridges[-1]["to"], tower["at"]) < 20.0
    assert any("chapel_at" in b for b in bridges)


def test_rhone_and_streets(city: dict) -> None:
    """Fine Rhône copied; XIXth-c. percées and boulevards excluded from the OSM streets."""
    assert city["fine_rivers"] == ["Rhône"]
    assert any(
        w["origin"] == "rivers_fine" and w["name"] == "Rhône" for w in city["waters"]
    )
    osm = [s for s in city["streets"] if s["origin"] == "osm"]
    assert len(osm) > 300
    names = {s["name"] for s in osm}
    assert {"Rue des Teinturiers", "Rue de la Carreterie", "Rue des Lices"} <= names
    assert not names & {
        "Rue de la République",
        "Cours Jean Jaurès",
        "Rue Thiers",
        "Boulevard Raspail",
        "Boulevard Limbert",
    }
    assert not any(n.startswith("Rue du Rempart") for n in names)
