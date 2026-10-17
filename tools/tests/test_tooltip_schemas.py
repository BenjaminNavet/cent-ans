"""Validates data/ui/tooltip_style.json and data/ui/tooltips.json (chantier IB, ADR 0109)."""

import json
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


@pytest.mark.parametrize("block", ["effects", "stats", "gauges"])
def test_every_rule_entry_has_a_text(block: str) -> None:
    """IB2/IB4: each effect, stat and gauge label emitted as an ib:rule: link has a bubble text.

    The labels moved out of `rich_tooltip.gd` (IB2): `data/ui/tooltips.json` is now the sole
    source, so this checks the data file is internally consistent (every entry has a body).
    """
    entries = _load("ui/tooltips.json")[block]
    missing = sorted(key for key, entry in entries.items() if not entry.get("body"))
    assert not missing, f"{block} without text in tooltips.json: {missing}"


def test_chain_delay_is_faster_than_idle_hover() -> None:
    """Holding Alt opens child bubbles faster than a plain hover."""
    chain = _load("ui/tooltip_style.json")["chain"]
    assert chain["hover_delay_s"] < chain["idle_hover_delay_s"]
