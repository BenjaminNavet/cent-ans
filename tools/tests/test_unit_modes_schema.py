"""Validates the battle unit mode rules against their schema (CB2)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "unit_modes.json").read_text(encoding="utf-8"))


def test_mangonel_breaches_less_than_the_heavy_engines() -> None:
    """Research CB4 § 6: the mangonel, a lighter engine, batters walls less."""
    damage = _rules()["breach"]["wall_damage_multiplier"]
    assert damage["unit_mangonel"] < damage["default"]


def test_breach_multipliers_name_known_unit_types() -> None:
    """Every per-type breach multiplier names a unit type of `data/unit_types/`."""
    damage = _rules()["breach"]["wall_damage_multiplier"]
    for key in damage:
        if key != "default":
            assert (DATA / "unit_types" / f"{key}.json").exists(), key


def test_skirmish_retreat_is_longer_than_its_minimum() -> None:
    """A skirmish hop is never shorter than the least retreat worth taking."""
    skirmish = _rules()["skirmish"]
    assert skirmish["retreat_m"] > skirmish["min_retreat_m"]
