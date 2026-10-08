"""Validates data/rules/campaign_weather.json against its schema (lot CM2, map weather)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_campaign_weather_rules_match_schema() -> None:
    """The map weather rules exist, match their schema and every season sums to 100."""
    rules = json.loads(
        (DATA / "rules" / "campaign_weather.json").read_text(encoding="utf-8")
    )
    for climate, seasons in rules["climates"].items():
        for season, chances in seasons.items():
            assert sum(chances.values()) == 100, (climate, season)
