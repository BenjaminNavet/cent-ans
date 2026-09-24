"""Validates data/ai/alignment.json against ai_alignment.schema.json (G4)."""

import json
from pathlib import Path

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_ai_alignment_matches_the_schema() -> None:
    """The AI side-change tuning file exists and is valid."""
    validator = schema_validator(DATA, "ai_alignment.schema.json")
    tuning = json.loads((DATA / "ai" / "alignment.json").read_text(encoding="utf-8"))
    errors = [error.message for error in validator.iter_errors(tuning)]
    assert not errors, errors


def test_a_grudge_is_a_negative_sum() -> None:
    """A grievance is a sum of negative opinion modifiers, never a positive one."""
    tuning = json.loads((DATA / "ai" / "alignment.json").read_text(encoding="utf-8"))
    assert tuning["grievance"]["grudge"] < 0
