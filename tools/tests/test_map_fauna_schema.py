"""Validates the ME2 fauna data against its schema and cross-checks species and catalogue ids."""

import json
from pathlib import Path

from conftest import assert_matches_schema

DATA = Path(__file__).resolve().parents[2] / "data"


def test_map_fauna_matches_schema() -> None:
    """``data/map/map_fauna.json`` matches ``map_fauna.schema.json``."""
    assert_matches_schema("map/map_fauna.json", "map_fauna.schema.json")


def test_map_fauna_references_resolve() -> None:
    """Species ids exist in the map-extra catalogue, zone species and gaits exist."""
    doc = json.loads((DATA / "map" / "map_fauna.json").read_text(encoding="utf-8"))
    catalog = {
        e["id"]
        for e in json.loads(
            (DATA / "art" / "dn_catalog_map_extra.json").read_text(encoding="utf-8")
        )
    }
    assert set(doc["species"]) <= catalog
    for entry in doc["species"].values():
        assert entry["gait"] in doc["render"]["gaits"]
        assert entry["herd_size"][0] <= entry["herd_size"][1]
    for zone in doc["zones"]:
        for item in zone["species"]:
            assert item["species"] in doc["species"], (zone["id"], item["species"])
