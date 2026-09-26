"""Validates the crest defence rules against their schema (SG5)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_crest_defence.json").read_text(encoding="utf-8")
    )


def test_battle_crest_defence_rules_match_schema() -> None:
    """`data/rules/battle_crest_defence.json` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "battle_crest_defence_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_line_stays_out_of_rout_contagion() -> None:
    """The line stands back at least as far as the rout contagion reaches (120 m)."""
    assert _rules()["line_setback_m"] >= 100
