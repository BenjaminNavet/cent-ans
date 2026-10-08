"""Validates the map flames and wind parameters against their schema (lot AS5)."""

from pathlib import Path

from conftest import assert_matches_schema

DATA = Path(__file__).resolve().parents[2] / "data"


def test_map_fire_wind_matches_schema() -> None:
    """data/fx/map_fire_wind.json matches fx_map_fire_wind.schema.json."""
    assert_matches_schema("fx/map_fire_wind.json", "fx_map_fire_wind.schema.json")
