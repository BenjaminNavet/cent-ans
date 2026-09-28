"""Validates the siege capture point rules against their schema (TW2 T4)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "siege_capture.json").read_text(encoding="utf-8")
    )


def test_siege_capture_rules_match_schema() -> None:
    """`data/rules/siege_capture.json` matches `siege_capture_rules.schema.json`."""
    schema = json.loads(
        (DATA / "schemas" / "siege_capture_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_the_square_is_the_longer_hold() -> None:
    """The victory point takes longer to hold than the gate, and falls back faster than it grows."""
    rules = _rules()
    assert rules["square"]["hold_s"] >= rules["gate"]["hold_s"]
    for point in ("square", "gate"):
        assert rules[point]["retake_per_s"] >= rules[point]["decay_per_s"]


def test_last_stand_only_helps_the_defenders() -> None:
    """The last stand softens the morale losses; it never makes them worse."""
    stand = _rules()["last_stand"]
    assert 0 < stand["loss_morale_factor"] <= 1
    assert 0 <= stand["contagion_factor"] <= 1
