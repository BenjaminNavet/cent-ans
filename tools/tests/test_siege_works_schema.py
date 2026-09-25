"""Validates the siege works rules (wall and gate HP, ram damage) against their schema (SG3)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "siege_works.json").read_text(encoding="utf-8"))


def test_siege_works_rules_match_schema() -> None:
    """`data/rules/siege_works.json` matches `siege_works_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "siege_works_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_wooden_gate_is_weaker_than_stone_walls() -> None:
    """At every fortification level the gate has far fewer HP than a wall piece."""
    rules = _rules()
    for level in range(6):
        gate = rules["gate"]["hp_base"] + level * rules["gate"]["hp_per_fortification"]
        wall = rules["wall"]["hp_base"] + level * rules["wall"]["hp_per_fortification"]
        assert 2 * gate < wall, level
