"""Validates the battle decision rules against their schema (lot EP9)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_battle_decision_matches_schema() -> None:
    """The battle decision file matches its schema."""
    schema = _load(DATA / "schemas" / "battle_decision_rules.schema.json")
    document = _load(DATA / "rules" / "battle_decision.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_an_army_without_its_general_breaks_sooner() -> None:
    """Losing the general can only raise the break threshold."""
    rules = _load(DATA / "rules" / "battle_decision.json")
    assert rules["break_share_without_general"] >= rules["break_share"]
