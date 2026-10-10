"""Validates the siege works rules (wall and gate HP, ram damage) against their schema (SG3)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _rules() -> dict:
    return json.loads((DATA / "rules" / "siege_works.json").read_text(encoding="utf-8"))


def test_wooden_gate_is_weaker_than_stone_walls() -> None:
    """At every fortification level the gate has far fewer HP than a wall piece."""
    rules = _rules()
    for level in range(6):
        gate = rules["gate"]["hp_base"] + level * rules["gate"]["hp_per_fortification"]
        wall = rules["wall"]["hp_base"] + level * rules["wall"]["hp_per_fortification"]
        assert 2 * gate < wall, level


def test_tw_siege_blocks_are_validated() -> None:
    """TW siege (ADR 0329) : les blocs tour, sortie, contre-batterie et second assaut sont exigés et bornés."""
    import copy

    import jsonschema

    schema = json.loads((DATA / "schemas" / "siege_works_rules.schema.json").read_text(encoding="utf-8"))
    rules = _rules()
    jsonschema.validate(rules, schema)
    for block in ("tower", "sortie", "counter_battery", "second_assault"):
        broken = copy.deepcopy(rules)
        del broken[block]
        try:
            jsonschema.validate(broken, schema)
        except jsonschema.ValidationError:
            pass
        else:
            raise AssertionError(block)
    bad = copy.deepcopy(rules)
    bad["tower"]["range_m"] = -1
    try:
        jsonschema.validate(bad, schema)
    except jsonschema.ValidationError:
        return
    raise AssertionError("negative tower range accepted")
