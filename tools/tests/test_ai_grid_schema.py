"""Validates data/ai/grid.json against ai_grid.schema.json (M3)."""

import json
from pathlib import Path

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_ai_grid_matches_the_schema() -> None:
    """The AI grid tuning file exists and is valid."""
    validator = schema_validator(DATA, "ai_grid.schema.json")
    tuning = json.loads((DATA / "ai" / "grid.json").read_text(encoding="utf-8"))
    errors = [error.message for error in validator.iter_errors(tuning)]
    assert not errors, errors
