"""Validates the DN-PAYS countryside data against its schema and cross-checks references."""

import json
from pathlib import Path

from conftest import assert_matches_schema

DATA = Path(__file__).resolve().parents[2] / "data"


def _doc() -> dict:
    return json.loads((DATA / "map" / "map_countryside.json").read_text(encoding="utf-8"))


def test_map_countryside_matches_schema() -> None:
    """``data/map/map_countryside.json`` matches ``map_countryside.schema.json``."""
    assert_matches_schema("map/map_countryside.json", "map_countryside.schema.json")


def test_map_countryside_references_resolve() -> None:
    """Rule props, regions and external region ids exist."""
    doc = _doc()
    for rule in doc["rules"]:
        for prop in rule["props"]:
            assert prop in doc["props"], (rule["id"], prop)
        for region in rule.get("regions", []):
            assert region in doc["regions"], (rule["id"], region)
        source = rule.get("regions_from")
        if source:
            sites = json.loads((DATA / source["file"]).read_text(encoding="utf-8"))[source["key"]]
            known = {site["id"] for site in sites}
            assert set(source.get("ids", [])) <= known, rule["id"]
        for biome in rule.get("filters", {}).get("biomes", []):
            assert biome in doc["biome_index"], (rule["id"], biome)
        assert rule.get("regions") or source or rule["mode"] in {"scatter", "road", "village"}


def test_map_countryside_models_are_in_a_dn_catalogue() -> None:
    """Every prop model id is a catalogue entry (glb shipped as a release package, not by git)."""
    known: set[str] = set()
    for path in (DATA / "art").glob("dn_catalog_*.json"):
        content = json.loads(path.read_text(encoding="utf-8"))
        entries = content if isinstance(content, list) else [e for v in content.values() if isinstance(v, list) for e in v]
        known |= {e["id"] for e in entries if isinstance(e, dict) and "id" in e}
    for prop_id, prop in _doc()["props"].items():
        assert prop["model"].split("/", 1)[1] in known, (prop_id, prop["model"])
