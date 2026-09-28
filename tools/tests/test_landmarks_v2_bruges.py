"""Bruges around 1340 (lot VH8): key dated facts of data/landmarks_v2/bruges.json."""

import json
from pathlib import Path

import pytest

from cent_ans_tools.geo import landmarks_v2

CITY = Path(__file__).resolve().parents[2] / "data" / "landmarks_v2" / "bruges.json"


def _city() -> dict:
    return json.loads(CITY.read_text(encoding="utf-8"))


def _present(item: dict, year: int) -> bool:
    return item.get("from_year", -9999) <= year <= item.get("until_year", 9999)


def test_bruges_vesten_1340() -> None:
    """1297 enclosure (≈ 6.8 km) dismantled around 1328: in 1340 only low passages, gates rebuilt 1361-1407."""
    walls = {w["id"]: w for w in _city()["walls"]}
    vesten = walls["vesten"]
    assert vesten["closed"] and "from_year" not in vesten
    assert 6500.0 < landmarks_v2.wall_length({"walls": [vesten]}) < 7500.0
    assert vesten["height_m"] <= 8.0 and not vesten.get("tower_spacing_m")  # earth bank
    gates_1340 = [g for g in vesten["gates"] if _present(g, 1340)]
    names = " ".join(g["name"] for g in gates_1340)
    for gate in (
        "Gentpoort",
        "Kruispoort",
        "Smedenpoort",
        "Ezelpoort",
        "Boeveriepoort",
        "Katelijnepoort",
    ):
        assert gate in names, gate
    rebuilt = [g for g in gates_1340 if not g["name"].startswith("Dampoort")]
    assert all(g["height_m"] <= 8.0 for g in rebuilt)  # dismantled around 1328

    def first_rebuild(prefix: str) -> int:
        return min(
            g["from_year"]
            for g in vesten["gates"]
            if g["name"].startswith(prefix) and g["height_m"] > 8.0 and "from_year" in g
        )

    assert first_rebuild("Gentpoort") == 1361
    assert first_rebuild("Boeveriepoort") == 1366
    assert first_rebuild("Smedenpoort") == 1367
    assert first_rebuild("Ezelpoort") == 1369
    # Relecture 2026-09-28 : Ezelpoort sur le bâtiment de 1369 (OSM (-561, 865), dans le fossé).
    ezel = next(g for g in vesten["gates"] if g["name"].startswith("Ezelpoort"))
    assert abs(ezel["at"][0] + 561) + abs(ezel["at"][1] - 865) < 40
    assert first_rebuild("Katelijnepoort") == 1401
    # Gentpoort and Kruispoort destroyed by the Ghent militia in 1382, rebuilt from 1401.
    for prefix in ("Gentpoort", "Kruispoort"):
        states = [g for g in vesten["gates"] if g["name"].startswith(prefix)]
        assert [g["height_m"] <= 8.0 for g in states if _present(g, 1390)] == [True]
        assert [g["height_m"] > 8.0 for g in states if _present(g, 1410)] == [True]
    # Stone walls of Jan van Oudenaerde: Begijnenvest (1398, five towers), Minnewater (1399).
    assert walls["mur_begijnenvest"]["from_year"] == 1398
    assert walls["mur_minnewater_katelijnepoort"]["from_year"] == 1399
    for key in ("mur_begijnenvest", "mur_minnewater_katelijnepoort"):
        assert not walls[key]["closed"]


def test_bruges_monuments_dated() -> None:
    """Belfry stages, halls, Waterhalle, town hall, St Saviour's fire, absent later buildings."""
    monuments = {m["id"]: m for m in _city()["monuments"]}
    belfries = [
        m
        for i, m in monuments.items()
        if i.startswith("beffroi_") and _present(m, 1340)
    ]
    assert [m["id"] for m in belfries] == ["beffroi_1340"]
    assert monuments["beffroi_1340"]["until_year"] == 1344
    assert (
        monuments["beffroi_1346"]["from_year"] == 1345
    )  # second square stage c. 1345-1346
    assert monuments["beffroi_1486"]["from_year"] == 1486  # octagon 1482-1486
    top = monuments["beffroi_1486"]["params"]
    assert top["height"] + top["size"] * 1.3 == pytest.approx(83, abs=2)  # 83 m today
    wh = monuments["waterhalle"]
    assert wh["params"]["length_m"] == 95 and wh["params"]["width_m"] == 24
    assert wh["until_year"] == 1787 and _present(wh, 1340)
    assert monuments["stadhuis"]["from_year"] == 1376
    assert monuments["ghyselhuus"]["until_year"] == 1375
    assert monuments["saint_donatien"]["until_year"] == 1799
    assert monuments["saint_sauveur_nef_romane"]["until_year"] == 1358
    # Relecture 2026-09-28 : nef gothique du premier quart du XVe s. (Inventaire 29716).
    assert monuments["saint_sauveur_nef"]["from_year"] == 1425
    assert wh["params"]["height_m"] == 30  # hauteur estimée (Wikipédia NL)
    assert monuments["poertoren"]["from_year"] == 1398
    assert monuments["poertoren"]["params"]["height"] == 18
    for later in (
        "oosterlingenhuis",
        "ter_beurze",
        "prinsenhof",
        "poortersloge",
        "sint_anna",
    ):
        assert (
            later not in monuments
        )  # after 1340 (Hanse house 1478, Ter Beurze 1453...)
    for m in monuments.values():
        assert m.get("certainty") in {"attested", "probable", "hypothetical"}, m["id"]
        assert m.get("description"), m["id"]


def test_bruges_waters_and_streets() -> None:
    """Reien drawn by hand from OSM (no fine river); no post-medieval canals or ring roads."""
    city = _city()
    assert city["fine_rivers"] == []
    assert all(w["origin"] in {"hand", "osm"} and w["draw"] for w in city["waters"])
    names = " ".join(w["name"] for w in city["waters"])
    for rei in (
        "Dijver",
        "Groenerei",
        "Spiegelrei",
        "Minnewater",
        "Kraanrei",
        "vesten",
    ):
        assert rei in names, rei
    for later in ("Coupure", "Boudewijn", "Kanaal", "Handelsdok"):
        assert later not in names, later  # 1751, 1896-1907, 1613-1623...
    osm = {s["name"] for s in city["streets"] if s["origin"] == "osm"}
    assert len([s for s in city["streets"] if s["origin"] == "osm"]) > 300
    assert {"Steenstraat", "Vlamingstraat", "Langestraat", "Katelijnestraat"} <= osm
    assert not osm & {"Koningin Elisabethlaan", "Coupure", "Kazernevest", "Komvest"}
    districts = {d["id"]: d for d in city["districts"]}
    assert (
        districts["ville_premiere_enceinte"]["density"]
        > districts["ville_accrue_1297"]["density"]
    )
