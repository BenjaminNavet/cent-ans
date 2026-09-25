"""Validates data/rules/campaign_weather.json against its schema (lot CM2, map weather)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_campaign_weather_rules_match_schema() -> None:
    """The map weather rules exist, match their schema and every season sums to 100."""
    schema = json.loads(
        (DATA / "schemas" / "campaign_weather_rules.schema.json").read_text(
            encoding="utf-8"
        )
    )
    rules = json.loads(
        (DATA / "rules" / "campaign_weather.json").read_text(encoding="utf-8")
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(rules), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]
    for climate, seasons in rules["climates"].items():
        for season, chances in seasons.items():
            assert sum(chances.values()) == 100, (climate, season)
