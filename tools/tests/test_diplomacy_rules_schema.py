"""Validates data/rules/diplomacy.json against diplomacy_rules.schema.json (lot RS-C)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_diplomacy_rules_match_schema() -> None:
    """The diplomacy rules file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "diplomacy_rules.schema.json")
    rules = _load(DATA / "rules" / "diplomacy.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_unknown_motive_is_rejected() -> None:
    """A cap on a motive the simulation does not know is a schema error."""
    schema = _load(DATA / "schemas" / "diplomacy_rules.schema.json")
    rules = {"opinion_caps": {"flattery": 10}}
    assert list(Draft202012Validator(schema).iter_errors(rules))


def test_marriage_and_herald_are_capped() -> None:
    """RS-C: the two motives seen stacking in the traces carry a cap."""
    rules = _load(DATA / "rules" / "diplomacy.json")
    assert {"marriage", "herald_embassy"} <= set(rules["opinion_caps"])
