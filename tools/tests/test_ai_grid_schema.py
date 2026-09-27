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


def test_ai_stances_are_tuned() -> None:
    """CV3-6: the AI stance and encounter sections are present and on."""
    tuning = json.loads((DATA / "ai" / "grid.json").read_text(encoding="utf-8"))
    postures = tuning["postures"]
    ambush = postures["ambush"]
    assert 0 < ambush["weight_permille"] <= 1000
    assert ambush["min_power_ratio"] < ambush["max_power_ratio"] <= 1.0
    assert postures["forced_march"]["enabled"]
    assert postures["entrenched"]["enabled"]
    assert 0 < tuning["encounters"]["detour_permille"] <= 1000


def test_schema_rejects_unknown_stance_keys() -> None:
    """CV3-6: a misspelt stance setting is refused."""
    validator = schema_validator(DATA, "ai_grid.schema.json")
    tuning = json.loads((DATA / "ai" / "grid.json").read_text(encoding="utf-8"))
    tuning["postures"]["ambush"]["weight"] = 3
    assert list(validator.iter_errors(tuning))
