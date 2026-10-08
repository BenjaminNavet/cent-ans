"""Validates data/rules/diplomacy.json against diplomacy_rules.schema.json (lot RS-C)."""

import json
from pathlib import Path

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def test_unknown_motive_is_rejected() -> None:
    """A cap on a motive the simulation does not know is a schema error."""
    validator = schema_validator(DATA, "diplomacy_rules.schema.json")
    assert list(validator.iter_errors({"opinion_caps": {"flattery": 10}}))


def test_marriage_and_herald_are_capped() -> None:
    """RS-C: the two motives seen stacking in the traces carry a cap."""
    rules = _load(DATA / "rules" / "diplomacy.json")
    assert {"marriage", "herald_embassy"} <= set(rules["opinion_caps"])
