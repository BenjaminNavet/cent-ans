"""Validates the relief pyramid manifest and the detail zones (chantier ZG, ADR 0036)."""

import json
from pathlib import Path

import pytest
from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"


@pytest.mark.parametrize(
    ("schema_name", "data_path"),
    [
        ("relief_pyramid.schema.json", "map/relief_pyramid.json"),
        ("detail_zones.schema.json", "map/detail_zones.json"),
    ],
)
def test_matches_schema(schema_name: str, data_path: str) -> None:
    """The file exists and matches its schema."""
    schema = json.loads((DATA / "schemas" / schema_name).read_text(encoding="utf-8"))
    document = json.loads((DATA / data_path).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_levels_halve_meters_per_px() -> None:
    """Each level is exactly twice as fine as the previous one."""
    manifest = json.loads(
        (DATA / "map" / "relief_pyramid.json").read_text(encoding="utf-8")
    )
    levels = sorted(manifest["levels"], key=lambda level: level["level"])
    for coarse, fine in zip(levels, levels[1:], strict=False):
        assert fine["level"] == coarse["level"] + 1
        assert fine["meters_per_px"] == pytest.approx(
            coarse["meters_per_px"] / 2, rel=1e-4
        )
