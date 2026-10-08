"""Validates data/rules/armies.json and siege_engines.json against their schemas (lot NT5, ADR 0128)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_army_rules_match_schema() -> None:
    """The army cap file matches its schema; the cap is 40 as in battle."""
    assert _load(DATA / "rules" / "armies.json")["max_units"] == 40


def test_engines_are_distinct_and_towers_name_a_tower_unit() -> None:
    """Engine ids are unique, ladders come first, a tower names a `wall_assault` unit type."""
    engines = _load(DATA / "rules" / "siege_engines.json")["engines"]
    ids = [engine["id"] for engine in engines]
    assert len(ids) == len(set(ids))
    assert engines[0]["kind"] == "ladders"
    for engine in engines:
        if engine["kind"] == "tower":
            unit = _load(DATA / "unit_types" / f"{engine['tower_unit_type']}.json")
            assert "wall_assault" in unit.get("abilities", []), engine["id"]
