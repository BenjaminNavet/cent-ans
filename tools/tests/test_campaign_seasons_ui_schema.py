"""Validates data/ui/campaign_seasons.json (lot TB1: seasons visible on the campaign map)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_campaign_seasons_match_schema() -> None:
    """The seasonal rendering settings match their schema."""
    schema = _load("schemas/campaign_seasons_ui.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("ui/campaign_seasons.json"))
    )
    assert not errors, [error.message for error in errors]


def test_winter_sea_is_darker_and_greyer() -> None:
    """The winter sea is darker and greyer than the summer sea."""
    seasons = _load("ui/campaign_seasons.json")["seasons"]
    winter, summer = seasons["winter"]["sea"], seasons["summer"]["sea"]
    assert sum(winter["tint"]) < sum(summer["tint"]) - 0.5
    assert winter["desaturate"] > summer["desaturate"] + 0.3


def test_season_grades_follow_the_art_bible() -> None:
    """Spring is cool, summer and autumn warm, winter blue and desaturated (bible § 12.6)."""
    seasons = _load("ui/campaign_seasons.json")["seasons"]
    grades = {name: season["grade"] for name, season in seasons.items()}
    assert grades["spring"]["temperature"] < 0
    assert grades["autumn"]["temperature"] > grades["summer"]["temperature"] > 0
    assert grades["winter"]["temperature"] < grades["spring"]["temperature"]
    assert grades["winter"]["saturation"] < 0.9
