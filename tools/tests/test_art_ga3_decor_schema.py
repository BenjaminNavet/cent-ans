"""Validates the GA3 generated decor catalogue against its schema (lot GA3-L1)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _document() -> dict:
    return json.loads((DATA / "art" / "ga3_decor.json").read_text(encoding="utf-8"))


def test_ga3_decor_matches_schema() -> None:
    """``data/art/ga3_decor.json`` matches ``art_ga3_decor.schema.json``."""
    schema = json.loads(
        (DATA / "schemas" / "art_ga3_decor.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_document()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_ga3_decor_ids_unique_and_caps() -> None:
    """Ids are unique; buildings stay under 8 k triangles (church excepted), props under 3 k."""
    objects = _document()["objects"]
    ids = [o["id"] for o in objects]
    assert len(ids) == len(set(ids))
    for entry in objects:
        cap = 30000 if entry["id"] == "church" else 8000
        if entry["kit_kind"] not in ("cottage", "church", "windmill"):
            cap = 3000
        assert entry["lod0"] <= cap, entry["id"]
