"""Validates the naval data (lot NV1, ADR 0028) against its schemas."""

import json
from pathlib import Path

import pytest

DATA = Path(__file__).resolve().parents[2] / "data"
NAVAL = DATA / "naval"


@pytest.mark.parametrize(
    "path", sorted((NAVAL / "ships").glob("*.json")), ids=lambda p: p.stem
)
def test_ship_classes_match_schema(path: Path) -> None:
    """Every ship class matches ship_class.schema.json and is named after its id."""
    ship = json.loads(path.read_text(encoding="utf-8"))
    assert ship["id"] == path.stem


def test_fleets_match_schema_and_reference_known_ships() -> None:
    """Fleets use known factions and ship classes."""
    fleets = json.loads((NAVAL / "fleets.json").read_text(encoding="utf-8"))
    ships = {p.stem for p in (NAVAL / "ships").glob("*.json")}
    factions = {p.stem for p in (DATA / "factions").glob("*.json")}
    for fleet in fleets["fleets"]:
        assert fleet["faction"] in factions
        assert set(fleet["ships"]) <= ships
