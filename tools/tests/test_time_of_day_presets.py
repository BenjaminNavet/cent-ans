"""PO4: battle time-of-day presets of data/fx/atmosphere.json (ADR 0097, DA bible 12.6)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_time_of_day_presets_complete() -> None:
    """The three presets exist and every sky they reference exists."""
    atmosphere = _load("fx/atmosphere.json")
    presets = atmosphere["time_of_day"]
    assert set(presets) == {"morning", "midday", "evening"}
    skies = atmosphere["skies"]
    for key, preset in presets.items():
        for weather, season_map in preset.get("sky", {}).items():
            assert weather in atmosphere["battle"], (key, weather)
            for season, sky_id in season_map.items():
                assert sky_id in skies, (key, weather, season, sky_id)


def test_every_core_phase_has_a_preset() -> None:
    """Each day phase of the core (EP8) maps to exactly one preset."""
    phases = [
        phase["key"] for phase in _load("rules/battle_time_of_day.json")["phases"]
    ]
    presets = _load("fx/atmosphere.json")["time_of_day"]
    for phase in phases:
        owners = [key for key, preset in presets.items() if phase in preset["phases"]]
        assert len(owners) == 1, (phase, owners)


def test_weather_sun_factors() -> None:
    """Each battle weather scales the sun of the hour."""
    for weather, look in _load("fx/atmosphere.json")["battle"].items():
        for field in ("sun_energy_scale", "sun_elevation_scale", "sun_tint"):
            assert field in look, (weather, field)
