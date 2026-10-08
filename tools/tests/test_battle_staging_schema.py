"""Validates the files of lot EP8 (time of day rules, battle staging) against their schemas."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_phases_cover_the_day_and_draw_names_exist() -> None:
    """Every hour falls in exactly one phase; the campaign draw names known phases."""
    rules = _load("rules/battle_time_of_day.json")
    phases = rules["phases"]
    for step in range(24 * 4):
        hour = step / 4
        inside = [
            p
            for p in phases
            if (p["from_hour"] <= hour < p["to_hour"])
            or (
                p["from_hour"] > p["to_hour"]
                and (hour >= p["from_hour"] or hour < p["to_hour"])
            )
        ]
        assert len(inside) == 1, (hour, [p["key"] for p in inside])
    keys = {p["key"] for p in phases}
    assert {w["phase"] for w in rules["campaign_draw"]} <= keys


def test_keyframes_are_sorted() -> None:
    """Time-of-day keyframes are in increasing hour order."""
    hours = [
        k["hour"] for k in _load("fx/battle_staging.json")["time_of_day"]["keyframes"]
    ]
    assert hours == sorted(hours)
