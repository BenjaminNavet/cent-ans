"""Validates the battle order queue rules against their schema (CB-M3)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_queue.json").read_text(encoding="utf-8")
    )


def test_queue_bound_matches_the_spec() -> None:
    """Spec CB-M3: at most 8 queued orders per regiment."""
    assert _rules()["max_queued_orders"] == 8
