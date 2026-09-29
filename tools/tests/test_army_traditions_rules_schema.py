"""Validates data/rules/army_traditions.json against its schema (lot TW2-T5, ADR 0112)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _rules() -> dict:
    return _load(DATA / "rules" / "army_traditions.json")


def test_army_traditions_match_schema() -> None:
    """The army traditions file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "army_traditions_rules.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_rules()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_rank_thresholds_increase() -> None:
    """Ranks need ever more experience (3-4 ranks, spec § T5)."""
    thresholds = _rules()["experience"]["rank_thresholds"]
    assert 3 <= len(thresholds) <= 4
    assert thresholds == sorted(set(thresholds))


def test_every_branch_has_a_first_tier_and_no_gap() -> None:
    """Each of the five branches starts at tier 1 and has no missing tier."""
    rules = _rules()
    branches = {branch["id"] for branch in rules["branches"]}
    assert branches == {"march", "stewardship", "shooting", "assault", "discipline"}
    for branch in branches:
        tiers = sorted(t["tier"] for t in rules["traditions"] if t["branch"] == branch)
        assert tiers == list(range(1, len(tiers) + 1)), (branch, tiers)


def test_tradition_ids_are_unique() -> None:
    """Tradition ids are unique."""
    ids = [t["id"] for t in _rules()["traditions"]]
    assert len(ids) == len(set(ids))
