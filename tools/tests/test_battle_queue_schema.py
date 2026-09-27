"""Validates the battle order queue rules against their schema (CB-M3)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_queue.json").read_text(encoding="utf-8")
    )


def test_battle_queue_rules_match_schema() -> None:
    """`data/rules/battle_queue.json` matches `battle_queue_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "battle_queue_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_queue_bound_matches_the_spec() -> None:
    """Spec CB-M3: at most 8 queued orders per regiment."""
    assert _rules()["max_queued_orders"] == 8
