"""Validates the relief pyramid manifest and the detail zones (chantier ZG, ADR 0036)."""

import json
from pathlib import Path

import pytest

DATA = Path(__file__).resolve().parents[2] / "data"


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
