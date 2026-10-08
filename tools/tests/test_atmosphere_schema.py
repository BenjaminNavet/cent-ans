"""Validates data/fx/atmosphere.json (lot V3, A1-05: HDRI skies and colour grading)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_atmosphere_references_exist() -> None:
    """Every sky referenced by a weather or season exists, and every panorama file is present."""
    atmosphere = _load("fx/atmosphere.json")
    skies = atmosphere["skies"]
    for sky in skies.values():
        assert (ROOT / "game" / sky["path"].removeprefix("res://")).is_file(), sky[
            "path"
        ]
    for weather, look in atmosphere["battle"].items():
        for season, sky_id in look["sky"].items():
            assert sky_id in skies, (weather, season, sky_id)
    for season, look in atmosphere["campaign"]["seasons"].items():
        assert look["sky"] in skies, (season, look["sky"])
