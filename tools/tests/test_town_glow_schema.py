"""Validates the town halo and chimney smoke parameters against their schema (lot TS-T4)."""

from pathlib import Path

from conftest import assert_matches_schema

DATA = Path(__file__).resolve().parents[2] / "data"


def test_town_glow_matches_schema() -> None:
    """data/fx/town_glow.json matches fx_town_glow.schema.json."""
    assert_matches_schema("fx/town_glow.json", "fx_town_glow.schema.json")
