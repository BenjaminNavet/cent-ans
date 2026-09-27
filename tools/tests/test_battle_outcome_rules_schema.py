"""Validates data/rules/battle_outcome.json against battle_outcome_rules.schema.json (lot CV3-1)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_battle_outcome_rules_match_schema() -> None:
    """The battle outcome rules file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "battle_outcome_rules.schema.json")
    rules = _load(DATA / "rules" / "battle_outcome.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_victories_rank_above_defeats() -> None:
    """Heroic and decisive victories pay more prestige than any defeat."""
    classes = _load(DATA / "rules" / "battle_outcome.json")["classes"]
    worst_victory = min(classes[k]["prestige"] for k in ("heroic", "decisive"))
    best_defeat = max(
        classes[k]["prestige"] for k in ("honourable_defeat", "disaster", "defeat")
    )
    assert worst_victory > best_defeat
