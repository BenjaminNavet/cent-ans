"""Validates the dense siege town and street furniture rules against their schema (lot BR3)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"
MANIFEST = (
    Path(__file__).resolve().parents[2]
    / "game"
    / "assets"
    / "models"
    / "buildings"
    / "manifest.json"
)


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_siege_town_rules_match_schema() -> None:
    """data/rules/siege_town.json matches siege_town_rules.schema.json."""
    schema = _load(DATA / "schemas" / "siege_town_rules.schema.json")
    document = _load(DATA / "rules" / "siege_town.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_prop_footprints_cover_the_models() -> None:
    """Each prop footprint is at least as large as every kit model of its kind."""
    if not MANIFEST.exists():
        return
    footprints = _load(DATA / "rules" / "siege_town.json")["props"]["footprints"]
    for name, entry in _load(MANIFEST).items():
        kind = entry["kind"]
        if kind not in footprints or entry["ruined"]:
            continue
        assert entry["length"] <= footprints[kind]["length_m"] + 1e-6, name
        assert entry["depth"] <= footprints[kind]["depth_m"] + 1e-6, name
