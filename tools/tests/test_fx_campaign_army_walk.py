"""Validates data/fx/campaign_army_walk.json (lot AS2) against its schema."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_campaign_army_walk_matches_schema() -> None:
    """The AS2 settings file matches its schema."""
    schema = json.loads((DATA / "schemas/fx_campaign_army_walk.schema.json").read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    data = json.loads((DATA / "fx/campaign_army_walk.json").read_text(encoding="utf-8"))
    errors = list(Draft202012Validator(schema).iter_errors(data))
    assert not errors, [error.message for error in errors]
