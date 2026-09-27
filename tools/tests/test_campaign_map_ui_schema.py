"""Validates data/ui/campaign_map.json (lot CV3-5: TW-scale lord on the campaign map)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_campaign_map_ui_matches_schema() -> None:
    """The campaign map rendering settings match their schema."""
    schema = _load("schemas/campaign_map_ui.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("ui/campaign_map.json"))
    )
    assert not errors, [error.message for error in errors]


def test_lord_is_larger_than_his_escort() -> None:
    """The general stands out from the escort (TW-scale lord)."""
    assert _load("ui/campaign_map.json")["map"]["army_figure_scale"] > 1.0
