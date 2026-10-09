"""Validates the DN-CHAMPS field table against its schema and the data it reads."""

import json
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_fields_match_schema() -> None:
    """The table validates against `art_dn_fields.schema.json`."""
    schema = _load(DATA / "schemas" / "art_dn_fields.schema.json")
    jsonschema.validate(_load(DATA / "art" / "dn_fields.json"), schema)


def test_fields_reference_known_crops_and_models() -> None:
    """Crop keys exist in the ground tables, model ids in the DN manifest."""
    fields = _load(DATA / "art" / "dn_fields.json")
    mix = _load(DATA / "art" / "ground_biome_mix.json")
    landscapes = _load(DATA / "map" / "agri_landscapes.json")
    known = set()
    for biome in mix["biomes"].values():
        known.update(biome["crops"])
    for landscape in landscapes["landscapes"].values():
        known.update(landscape["crops"])
    assets = _load(DATA / "art" / "dn_manifest.json")["assets"]
    for material, crop in fields["crops"].items():
        assert material in known, material
        for entry in crop["variants"] + crop.get("accents", []):
            assert entry["id"].split("/")[1] in assets, entry["id"]
