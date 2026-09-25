"""Validates the attacker's archery duel rules against their schema (EP9b)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_duel.json").read_text(encoding="utf-8"))


def test_battle_duel_rules_match_schema() -> None:
    """`data/rules/battle_duel.json` matches `battle_duel_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "battle_duel_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_won_duel_lasts_longer_but_stays_bounded() -> None:
    """A won duel outlasts a lost one and stays under the refusal delay plus the limit."""
    rules = _rules()
    decision = json.loads(
        (DATA / "rules" / "battle_decision.json").read_text(encoding="utf-8")
    )
    assert rules["duel_limit_seconds"] <= rules["winning_duel_max_seconds"]
    assert (
        rules["winning_duel_max_seconds"]
        <= rules["duel_limit_seconds"] + decision["refusal_seconds"]
    )
