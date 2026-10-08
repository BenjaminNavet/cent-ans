"""Validates data/rules/crusade.json against its schema (lot JR1, ADR 0165)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rules() -> dict:
    return _load(DATA / "rules" / "crusade.json")


def test_provinces_exist_and_coast_is_holy_land() -> None:
    """The target and the Holy Land are real provinces; the coast is part of it."""
    rules = _rules()
    holy_land = set(rules["holy_land"])
    assert rules["target_province"] in holy_land
    assert set(rules["coastal_holy_land"]) <= holy_land
    for province in holy_land:
        assert (DATA / "provinces" / f"{province}.json").is_file(), province
    for province in rules["coastal_holy_land"]:
        assert _load(DATA / "provinces" / f"{province}.json")["coastal"], province


def test_units_exist_and_are_not_mercenaries() -> None:
    """Contingents and the starting army reuse existing, ordinary unit types."""
    rules = _rules()
    units = [entry["unit"] for entry in rules["passage"]["unit_table"]]
    assert len(units) == len(set(units)), "a unit type listed twice"
    for unit in units + rules["starting_army"]:
        path = DATA / "unit_types" / f"{unit}.json"
        assert path.is_file(), unit
        assert not _load(path).get("mercenary"), unit


def test_base_settlement_is_a_port() -> None:
    """The base is an existing port: the passage needs one."""
    base = _rules()["base_settlement"]
    settlements = [
        settlement
        for path in (DATA / "settlements").glob("prov_*.json")
        for settlement in _load(path)
    ]
    found = [s for s in settlements if s["id"] == base]
    assert found, base
    assert found[0].get("port"), base


def test_scale_is_consistent() -> None:
    """Thresholds are ordered and the spec's scale holds (§ 4.1)."""
    rules = _rules()
    zeal, desertion, fervor = rules["zeal"], rules["desertion"], rules["fervor"]
    assert desertion["threshold"] <= zeal["low_threshold"] < zeal["high_threshold"]
    assert fervor["target_floor"] >= zeal["low_threshold"]
    assert fervor["start"] > zeal["low_threshold"]
    passage = rules["passage"]
    assert passage["delay_turns"] <= passage["cooldown_turns"]
    assert passage["units_base"] <= passage["max_units"]
