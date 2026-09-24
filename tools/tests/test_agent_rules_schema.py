"""Validates data/rules/agents.json against agent_rules.schema.json (lot C6, campaign agents)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_agent_rules_match_schema() -> None:
    """The agent rules file exists and matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "agent_rules.schema.json").read_text(encoding="utf-8")
    )
    rules = json.loads((DATA / "rules" / "agents.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_agent_thresholds_increase() -> None:
    """Seal thresholds are strictly increasing."""
    rules = json.loads((DATA / "rules" / "agents.json").read_text(encoding="utf-8"))
    thresholds = rules["experience_thresholds"]
    assert thresholds == sorted(set(thresholds))
