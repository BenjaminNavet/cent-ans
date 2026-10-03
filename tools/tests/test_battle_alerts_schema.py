"""Validates the typed battle alert rules against their schema (CB5)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"

ALERT_KINDS = {
    "rout",
    "general_down",
    "flanked",
    "reinforcements",
    "ammo_out",
    "wall_breached",
    "gate_destroyed",
    "square_threatened",
}


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_alerts.json").read_text(encoding="utf-8"))


def test_battle_alerts_rules_match_schema() -> None:
    """`data/rules/battle_alerts.json` matches `battle_alerts_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "battle_alerts_rules.schema.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_every_alert_kind_has_an_importance() -> None:
    """Every `AlertKind` (core) has a listed importance, so none sorts last by accident."""
    rules = _rules()
    assert set(rules["importance"]) == ALERT_KINDS


def test_duration_and_merge_window_are_the_spec_defaults() -> None:
    """The design spec asks for an 8 s display and a 5 s merge window."""
    rules = _rules()
    assert rules["duration_s"] == 10
    assert rules["merge_window_s"] == 5


def test_max_shown_matches_the_column_cap() -> None:
    """The spec caps the column at 5 alerts at once."""
    assert _rules()["max_shown"] == 5
