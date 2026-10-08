"""Validates the map atmosphere parameters against their schema (lot ME5)."""

from conftest import assert_matches_schema


def test_map_atmosphere_matches_schema() -> None:
    """data/fx/map_atmosphere.json matches fx_map_atmosphere.schema.json."""
    assert_matches_schema("fx/map_atmosphere.json", "fx_map_atmosphere.schema.json")
