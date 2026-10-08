"""Validates the crest defence rules against their schema (SG5)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads(
        (DATA / "rules" / "battle_crest_defence.json").read_text(encoding="utf-8")
    )


def test_line_stays_out_of_rout_contagion() -> None:
    """The line stands back at least as far as the rout contagion reaches (120 m)."""
    assert _rules()["line_setback_m"] >= 100
