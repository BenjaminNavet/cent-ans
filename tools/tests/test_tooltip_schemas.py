"""Validates data/ui/tooltip_style.json and data/ui/tooltips.json (chantier IB, ADR 0109)."""

import json
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


def test_chain_delay_is_faster_than_idle_hover() -> None:
    """Holding Alt opens child bubbles faster than a plain hover."""
    chain = _load("ui/tooltip_style.json")["chain"]
    assert chain["hover_delay_s"] < chain["idle_hover_delay_s"]
