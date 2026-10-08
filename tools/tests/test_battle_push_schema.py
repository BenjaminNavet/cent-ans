"""Validates the continuous line push rules against their schema (EP11)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "battle_push.json").read_text(encoding="utf-8"))


def test_push_is_a_slow_continuous_recoil() -> None:
    """Lines give ground by a few metres per ten seconds, not at a run."""
    pressure = _rules()["pressure"]
    assert 0.1 <= pressure["max_speed_mps"] <= 1.0
    assert pressure["dead_band"] < pressure["full_speed_share"]


def test_compression_hurts_the_compressed_regiment() -> None:
    """A compressed regiment takes more and deals less; bulge rear share below 1."""
    rules = _rules()
    assert rules["compression"]["damage_taken_bonus"] > 0.0
    assert 0.0 < rules["compression"]["fighting_malus"] < 1.0
    assert rules["bulge"]["rear_share"] < 1.0
