"""Validates data/ui/tooltip_style.json and data/ui/tooltips.json (chantier IB, ADR 0109)."""

import json
import re
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


@pytest.mark.parametrize(
    ("data_file", "schema_file"),
    [
        ("ui/tooltip_style.json", "schemas/tooltip_style.schema.json"),
        ("ui/tooltips.json", "schemas/tooltips.schema.json"),
    ],
)
def test_tooltip_data_matches_schema(data_file: str, schema_file: str) -> None:
    """Each tooltip data file matches its schema."""
    schema = _load(schema_file)
    Draft202012Validator.check_schema(schema)
    errors = list(Draft202012Validator(schema).iter_errors(_load(data_file)))
    assert not errors, [error.message for error in errors]


@pytest.mark.parametrize("block", ["effects", "stats", "gauges"])
def test_every_rule_entry_has_a_text(block: str) -> None:
    """IB2/IB4: each effect, stat and gauge label emitted as an ib:rule: link has a bubble text.

    The labels moved out of `rich_tooltip.gd` (IB2): `data/ui/tooltips.json` is now the sole
    source, so this checks the data file is internally consistent (every entry has a body).
    """
    entries = _load("ui/tooltips.json")[block]
    missing = sorted(key for key, entry in entries.items() if not entry.get("body"))
    assert not missing, f"{block} without text in tooltips.json: {missing}"


def _gd_plain_keys() -> set[str]:
    """`ib:plain:<key>` keys referenced from GDScript (`RichTooltip.attach_plain`/`.plain_spec`)."""
    keys: set[str] = set()
    for path in (ROOT / "game/scripts").rglob("*.gd"):
        source = path.read_text(encoding="utf-8")
        keys.update(re.findall(r'attach_plain\([^,]+,\s*"([a-z][a-z0-9_]*)"', source))
    return keys


def test_every_plain_key_referenced_from_gdscript_exists() -> None:
    """IB2: each `RichTooltip.attach_plain(control, "<key>")` has a `plain` entry in tooltips.json."""
    entries = _load("ui/tooltips.json")["plain"]
    missing = sorted(key for key in _gd_plain_keys() if key not in entries)
    assert not missing, f"plain keys used in GDScript but missing from tooltips.json: {missing}"


def test_chain_delay_is_faster_than_idle_hover() -> None:
    """Holding Alt opens child bubbles faster than a plain hover."""
    chain = _load("ui/tooltip_style.json")["chain"]
    assert chain["hover_delay_s"] < chain["idle_hover_delay_s"]
