"""Validates the battle opening rules against their schema (CV3-2)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_opening.json").read_text(encoding="utf-8")
    )


def test_battle_opening_rules_match_schema() -> None:
    """`data/rules/battle_opening.json` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "battle_opening_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_flank_zone_is_a_band() -> None:
    """The ambusher's zone starts off the column and has some depth."""
    flank = _rules()["flank"]
    assert flank["far_m"] > flank["near_m"] + 40


def test_palisade_slows_horse_more_than_foot() -> None:
    """Horses cross the palisade more slowly than men on foot."""
    palisade = _rules()["palisade"]
    assert palisade["horse_crossing_factor"] <= palisade["foot_crossing_factor"]
