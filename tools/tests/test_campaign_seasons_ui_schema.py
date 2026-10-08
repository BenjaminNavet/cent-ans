"""Validates data/ui/campaign_seasons.json (lot TB1: seasons visible on the campaign map)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_winter_sea_is_steel_grey() -> None:
    """The winter sea is much greyer than the summer sea, towards a readable steel grey."""
    seasons = _load("ui/campaign_seasons.json")["seasons"]
    winter, summer = seasons["winter"]["sea"], seasons["summer"]["sea"]
    assert winter["grey_amount"] > summer["grey_amount"] + 0.3
    red, green, blue = winter["grey"]
    assert red < green < blue and 0.03 < green < 0.3


def test_season_grades_follow_the_art_bible() -> None:
    """Spring is cool, summer and autumn warm, winter blue and desaturated (bible § 12.6)."""
    seasons = _load("ui/campaign_seasons.json")["seasons"]
    grades = {name: season["grade"] for name, season in seasons.items()}
    assert grades["spring"]["temperature"] < 0
    assert grades["autumn"]["temperature"] > grades["summer"]["temperature"] > 0
    assert grades["winter"]["temperature"] < grades["spring"]["temperature"]
    assert grades["winter"]["saturation"] < 0.9
