"""Validates data/map/settlement_markers.json (lot DA3 ranks, lot DV2 shield above the name)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_settlement_markers_match_schema() -> None:
    """The catalogue matches its schema."""
    schema = _load("schemas/settlement_markers.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("map/settlement_markers.json"))
    )
    assert not errors, [error.message for error in errors]


def test_no_pictogram_left() -> None:
    """Lot DV2 (ADR 0124): no painted pictogram atlas any more."""
    catalog = _load("map/settlement_markers.json")
    for key in ("atlas", "pictograms", "kinds", "port_badge", "badge"):
        assert key not in catalog, key
    assert not (ROOT / "game/assets/map/markers/settlement_markers.png").exists()


def test_rank_sizes_cover_every_rank() -> None:
    """Each rank 1-4 has a size and ranks grow."""
    sizes = _load("map/settlement_markers.json")["size_px"]
    values = [sizes[str(rank)] for rank in range(1, 5)]
    assert values == sorted(values), values
