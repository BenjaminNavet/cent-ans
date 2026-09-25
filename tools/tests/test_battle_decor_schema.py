"""Validates the battlefield decor rules against their schema (lot EP6)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rules() -> dict:
    return _load(DATA / "rules" / "battle_decor.json")


def test_battle_decor_rules_match_schema() -> None:
    """``data/rules/battle_decor.json`` matches ``battle_decor_rules.schema.json``."""
    schema = _load(DATA / "schemas" / "battle_decor_rules.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_battle_decor_profiles_exist() -> None:
    """The default and terrain profiles name existing landscape profiles."""
    rules = _rules()
    profiles = rules["profiles"]
    assert rules["default_profile"] in profiles
    for terrain, profile in rules["terrain_profiles"].items():
        assert profile in profiles, terrain


def test_battle_decor_provinces_exist_once() -> None:
    """Every province of a profile exists and belongs to one profile only."""
    known = {path.stem for path in (DATA / "provinces").glob("prov_*.json")}
    seen: dict[str, str] = {}
    for name, profile in _rules()["profiles"].items():
        for province in profile["provinces"]:
            assert province in known, (name, province)
            assert province not in seen, (province, seen.get(province), name)
            seen[province] = name


def test_battle_decor_spans_are_ordered() -> None:
    """Every ``[min, max]`` pair is ordered."""

    def walk(node: object, path: str) -> None:
        if isinstance(node, dict):
            for key, value in node.items():
                if key == "prop_size_m":
                    continue  # [length, depth], not spans
                walk(value, f"{path}.{key}")
        elif (
            isinstance(node, list)
            and len(node) == 2
            and all(isinstance(v, int | float) for v in node)
        ):
            assert node[0] <= node[1], path

    walk(_rules(), "")


def test_battle_decor_plan_example_matches_schema() -> None:
    """The example decor plan (EP7 hand-made maps) matches its schema."""
    schema = _load(DATA / "schemas" / "battle_decor_plan.schema.json")
    Draft202012Validator.check_schema(schema)
    example = _load(DATA / "battle_maps" / "decor_plan_example.json")
    errors = sorted(
        Draft202012Validator(schema).iter_errors(example), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
