"""Validates data/rules/agents.json against agent_rules.schema.json (lot C6, campaign agents)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_agent_thresholds_increase() -> None:
    """Seal thresholds are strictly increasing."""
    rules = json.loads((DATA / "rules" / "agents.json").read_text(encoding="utf-8"))
    thresholds = rules["experience_thresholds"]
    assert thresholds == sorted(set(thresholds))
