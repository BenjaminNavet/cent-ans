"""Validates data/ui/next_hints.json (lot UX2: "what to do now" hint of the campaign map)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_next_hints_are_unique_and_end_with_fallback() -> None:
    """Each condition appears once and the always-true fallback comes last."""
    ids = [hint["id"] for hint in _load("ui/next_hints.json")["hints"]]
    assert len(ids) == len(set(ids))
    assert ids[-1] == "end_turn"
