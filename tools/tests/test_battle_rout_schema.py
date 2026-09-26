"""Validates the rout direction and contagion rules against their schema (EP10)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_rout.json").read_text(encoding="utf-8"))


def test_battle_rout_rules_match_schema() -> None:
    """`data/rules/battle_rout.json` matches `battle_rout_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "battle_rout_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_a_friend_behind_weighs_less_than_one_beside() -> None:
    """A routing friend behind counts less than one giving way beside; bands ordered."""
    contagion = _rules()["contagion"]
    assert contagion["behind_weight"] < 1.0
    assert contagion["beside_depth_m"] <= contagion["behind_depth_m"]
    assert contagion["behind_depth_m"] <= contagion["radius_m"]
