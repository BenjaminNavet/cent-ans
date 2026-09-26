"""Validates data/ui/ai_turn_replay.json (lot CT1: replay of the AI turn on the campaign map)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_ai_turn_replay_matches_schema() -> None:
    """The replay tuning file matches its schema."""
    schema = _load("schemas/ai_turn_replay.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("ui/ai_turn_replay.json"))
    )
    assert not errors, [error.message for error in errors]


def test_ai_turn_replay_durations_are_ordered() -> None:
    """The shortest move is not longer than the longest one, and speed x1 is offered."""
    tuning = _load("ui/ai_turn_replay.json")
    assert tuning["min_move_duration_s"] <= tuning["max_move_duration_s"]
    assert 1 in tuning["speeds"]
