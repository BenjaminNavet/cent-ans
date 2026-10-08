"""Consistency of `data/map/map_landmarks_extra.json` with the catalogue and the map (lot DN ME6)."""

import json
import math

from conftest import DATA

DECOR = json.loads(
    (DATA / "map" / "map_landmarks_extra.json").read_text(encoding="utf-8")
)
CATALOG_IDS = {
    e["id"]
    for e in json.loads(
        (DATA / "art" / "dn_catalog_map_extra.json").read_text(encoding="utf-8")
    )
}
SETTLEMENTS = json.loads(
    (DATA / "map" / "settlements_px.json").read_text(encoding="utf-8")
)


def test_types_use_catalogue_ids() -> None:
    """Every type points at an id of the DN map-extra catalogue."""
    missing = [
        t for t, spec in DECOR["types"].items() if spec["model"] not in CATALOG_IDS
    ]
    assert not missing, missing


def test_rules_and_sites_reference_known_types() -> None:
    """Rules and sites only use declared types."""
    types = set(DECOR["types"])
    for rule in DECOR["rules"]:
        assert set(rule["types"]) <= types, rule["id"]
    for site in DECOR["sites"]:
        assert site["type"] in types, site["id"]


def test_route_nodes_are_settlements() -> None:
    """Baked route nodes are real settlements."""
    for rule in DECOR["rules"]:
        if rule["mode"] == "route":
            assert len(rule["nodes"]) >= 2, rule["id"]
            assert all(node in SETTLEMENTS for node in rule["nodes"]), rule["id"]


def test_site_pixels_match_projection() -> None:
    """Rouen-like sanity: baked pixels lie on the map and far apart sites differ."""
    size = (7168, 6144)
    for site in DECOR["sites"]:
        if "px" in site:
            x, y = site["px"]
            assert 0 <= x < size[0] and 0 <= y < size[1], site["id"]
    carnac = next(s for s in DECOR["sites"] if s["id"] == "carnac_alignments")["px"]
    stonehenge = next(s for s in DECOR["sites"] if s["id"] == "stonehenge")["px"]
    # about 400 km apart on the ground
    assert 450 < math.dist(carnac, stonehenge) < 700
