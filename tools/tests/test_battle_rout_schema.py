"""Validates the rout direction and contagion rules against their schema (EP10)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_rout.json").read_text(encoding="utf-8"))


def test_a_friend_behind_weighs_less_than_one_beside() -> None:
    """A routing friend behind counts less than one giving way beside; bands ordered."""
    contagion = _rules()["contagion"]
    assert contagion["behind_weight"] < 1.0
    assert contagion["beside_depth_m"] <= contagion["behind_depth_m"]
    assert contagion["behind_depth_m"] <= contagion["radius_m"]
