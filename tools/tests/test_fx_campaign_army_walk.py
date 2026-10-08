"""Validates data/fx/campaign_army_walk.json (lot AS2) against its schema."""

from pathlib import Path

from conftest import assert_matches_schema

DATA = Path(__file__).resolve().parents[2] / "data"


def test_campaign_army_walk_matches_schema() -> None:
    """The AS2 settings file matches its schema."""
    assert_matches_schema(
        "fx/campaign_army_walk.json", "fx_campaign_army_walk.schema.json"
    )
