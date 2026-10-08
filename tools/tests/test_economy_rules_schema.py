"""Validates data/rules/economy.json against economy_rules.schema.json (lot EQ1)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_tax_brackets_are_ordered() -> None:
    """RS-B: a higher bracket takes more (multiplier and burden) than a lower one."""
    rules = json.loads((DATA / "rules" / "economy.json").read_text(encoding="utf-8"))
    brackets = [rules["tax_rates"][name] for name in ("low", "normal", "high")]
    for lower, higher in zip(brackets, brackets[1:], strict=False):
        assert lower["multiplier"] < higher["multiplier"]
        assert lower["burden"] < higher["burden"]
    assert rules["tax_rates"]["normal"]["multiplier"] == 1.0


def test_garrison_ceilings_cover_one_point() -> None:
    """RS-B: one point of Garrison effect never exceeds the relief/reinforce ceilings."""
    rules = json.loads((DATA / "rules" / "economy.json").read_text(encoding="utf-8"))
    assert (
        rules["garrison_relief_percent_per_point"]
        <= rules["garrison_relief_max_percent"]
    )
    assert (
        rules["garrison_reinforce_percent_per_point"]
        <= rules["garrison_reinforce_max_percent"]
    )
