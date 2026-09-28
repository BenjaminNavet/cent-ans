"""Validates data/rules/replenishment.json against its schema (lot TW2-T2, ADR 0102)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rules() -> dict:
    return _load(DATA / "rules" / "replenishment.json")


def test_replenishment_rules_match_schema() -> None:
    """The replenishment rules file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "replenishment_rules.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_territory_rates_are_ordered() -> None:
    """Own lands replenish most, hostile lands not at all (spec § T2)."""
    territory = _rules()["replenishment"]["territory_percent"]
    assert (
        territory["own"] >= territory["ally"] >= territory["neutral"] >= territory["hostile"]
    )
    assert territory["hostile"] == 0


def test_forced_march_replenishes_nothing() -> None:
    """A forced march brings no reinforcements."""
    assert _rules()["replenishment"]["stance_percent"]["forced_march"] == 0


def test_skills_and_traits_exist() -> None:
    """Every skill and trait named by the rules exists in the data."""
    rules = _rules()["replenishment"]
    for skill in rules["general_skill_percent"]:
        assert (DATA / "skills" / f"{skill}.json").is_file(), skill
    for trait in rules["general_trait_percent"]:
        assert (DATA / "traits" / f"{trait}.json").is_file(), trait
