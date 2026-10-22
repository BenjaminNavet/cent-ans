"""Validates data/fx/battle_impact.json (lot B2)."""

from conftest import assert_matches_schema


def test_battle_impact_matches_schema() -> None:
    """The melee impact file matches its schema."""
    assert_matches_schema("fx/battle_impact.json", "fx_battle_impact.schema.json")
