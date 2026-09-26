"""Paris at 1:1 from ALPAGE (lot VH5): data facts and the pure parts of ``geo.alpage``."""

import json
import math
import struct
from pathlib import Path

import pytest
from shapely.geometry import LineString, Point, Polygon
from shapely.strtree import STRtree

from cent_ans_tools.geo import alpage, landmarks_v2

DATA = Path(__file__).resolve().parents[2] / "data"
PARIS = DATA / "landmarks_v2" / "paris.json"


def _paris() -> dict:
    return json.loads(PARIS.read_text(encoding="utf-8"))


def test_gpkg_geometry_header() -> None:
    """GeoPackage blob: 8-byte header, optional envelope, then WKB."""
    point = Point(651000.0, 6862000.0)
    blob = b"GP" + bytes([0, 0b0000_0011]) + struct.pack("<i", 2154)
    blob += struct.pack("<4d", 0.0, 1.0, 0.0, 1.0) + point.wkb
    assert alpage._gpkg_geometry(blob).equals(point)


def test_frontage_on_street() -> None:
    """A strip parcel facing a street: frontage midpoint, inward normal, length, depth."""
    street = LineString([(-50.0, 0.0), (50.0, 0.0)])
    parcel = Polygon([(0.0, 3.0), (6.0, 3.0), (6.0, 33.0), (0.0, 33.0)])
    front = alpage._frontage(parcel, STRtree([street]), [street], [2.5], 4.0)
    assert front is not None
    mid, normal, length, depth = front
    assert mid[0] == pytest.approx(3.0) and mid[1] == pytest.approx(3.0)
    assert normal[1] == pytest.approx(1.0)
    assert length == pytest.approx(6.0) and depth == pytest.approx(30.0)
    # Far from any street: no frontage (interior parcel).
    far = Polygon([(0.0, 40.0), (6.0, 40.0), (6.0, 60.0), (0.0, 60.0)])
    assert alpage._frontage(far, STRtree([street]), [street], [2.5], 4.0) is None


def test_paris_file() -> None:
    """Paris v2: ALPAGE streets and parcels, credited under the ODbL."""
    city = _paris()
    assert sum(s["origin"] == "alpage" for s in city["streets"]) > 800
    assert len(city["parcels"]) > 2000
    assert all(len(p) == 5 for p in city["parcels"])
    alp = [s for s in city["sources"] if "ALPAGE" in s["title"]]
    assert alp and all(s["extracted"] and "ODbL" in s["license"] for s in alp)
    assert (
        city["fine_rivers"] == []
    )  # medieval Seine from ALPAGE instead of the fine river
    assert all(w["origin"] == "alpage" and "polygon" in w for w in city["waters"])


def test_paris_1340_facts() -> None:
    """Paris around 1340: dated walls, bridges and monuments."""
    city = _paris()
    walls = {w["id"]: w for w in city["walls"]}
    assert "from_year" not in walls["philippe_auguste_droite"]
    assert "from_year" not in walls["philippe_auguste_gauche"]
    assert walls["charles_v_levee"]["from_year"] == 1356
    assert walls["charles_v"]["from_year"] > 1356
    pa = landmarks_v2.wall_length(
        {"walls": [walls["philippe_auguste_droite"], walls["philippe_auguste_gauche"]]}
    )
    assert 4500.0 < pa < 6000.0, pa  # ≈ 5 100 m on both banks
    gates = [g["name"] for w in city["walls"] for g in w["gates"]]
    # Gate names of 1340 (ALPAGE gives those of 1380: Buci from 1352, Saint-Michel from 1394).
    for name in (
        "Porte Saint-Denis",
        "Porte Saint-Jacques",
        "Porte Saint-Germain (porte de Buci en 1352)",
        "Porte des Cordeliers",
        "Porte d'Enfer ou Gibard (porte Saint-Michel en 1394)",
    ):
        assert name in gates
    assert not any("Braque" in g for g in gates)
    bridges = {b["id"]: b for b in city["bridges"]}
    assert bridges["grand_pont"]["houses"] and bridges["petit_pont"]["houses"]
    assert bridges["pont_saint_michel"]["from_year"] == 1378
    petit = math.dist(bridges["petit_pont"]["from"], bridges["petit_pont"]["to"])
    assert 30.0 < petit < 55.0  # ≈ 40 m
    mon = {m["id"]: m for m in city["monuments"]}
    assert mon["notre_dame"]["params"]["length_m"] == pytest.approx(128, abs=4)
    assert mon["tour_horloge"]["from_year"] == 1350
    assert mon["bastille"]["from_year"] == 1370
    assert mon["louvre"]["until_year"] < mon["louvre_charles_v"]["from_year"] == 1364
    assert mon["sainte_chapelle"]["certainty"] == "attested"
    # Historian review: the Filles-Dieu reach this site in 1360; Sainte-Agnès is Saint-Eustache.
    assert mon["filles_dieu"]["from_year"] == 1360
    assert mon["sainte_agnes"]["name"].startswith("Saint-Eustache")
    spaces = {s["id"] for s in city["open_spaces"]}
    assert {
        "ile_notre_dame",
        "ile_aux_vaches",
        "greve",
        "grand_pre_aux_clercs",
    } <= spaces
    # The Cité and both banks inside Philippe Auguste's walls are districts.
    assert {"cite", "ville", "universite"} <= {d["id"] for d in city["districts"]}
