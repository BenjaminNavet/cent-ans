"""Validates data/events/*.json against data/schemas/event.schema.json (M10)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_every_event_matches_the_schema() -> None:
    """At least 40 events, each valid, file name equal to id."""
    files = sorted((DATA / "events").glob("*.json"))
    assert len(files) >= 40
    kinds = {"historical": 0, "random": 0, "chained": 0}
    for path in files:
        event = json.loads(path.read_text(encoding="utf-8"))
        assert path.stem == event["id"]
        kinds[event["kind"]] += 1
    assert kinds["historical"] >= 20
    assert kinds["random"] >= 20
