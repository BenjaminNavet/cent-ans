"""Validates data/rules/postures.json against posture_rules.schema.json (lot CV3-1, army stances)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_posture_rules_match_schema() -> None:
    """The posture rules file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "posture_rules.schema.json")
    rules = _load(DATA / "rules" / "postures.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_ambush_chance_bounds_are_ordered() -> None:
    """The ambush chance floor stays below its ceiling."""
    ambush = _load(DATA / "rules" / "postures.json")["ambush"]
    assert ambush["chance_min"] < ambush["chance_max"]


def test_scout_unit_types_exist() -> None:
    """Every scout unit type is a real unit type."""
    ambush = _load(DATA / "rules" / "postures.json")["ambush"]
    for unit_type in ambush["scout_unit_types"]:
        assert (DATA / "unit_types" / f"{unit_type}.json").is_file(), unit_type
