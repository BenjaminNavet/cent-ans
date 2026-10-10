"""Validates data/ui/transitions.json (lot TW trans: auto-resolve summary, history card, briefing)."""

import json
from pathlib import Path

import jsonschema

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_transitions_ui_matches_its_schema() -> None:
    """The texts file follows the schema and carries at least three tips."""
    data = _load("ui/transitions.json")
    jsonschema.validate(data, _load("schemas/transitions_ui.schema.json"))
    assert len(data["briefing"]["tips"]) >= 3
