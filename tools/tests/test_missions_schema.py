"""Validates data/missions.json against its schema (lot NT3, ADR 0127)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"

GOALS = {"control", "hold", "build", "treaty", "count"}


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _missions() -> dict:
    return _load(DATA / "missions.json")


def test_every_goal_has_a_template_with_unique_ids() -> None:
    """Every goal is offered; a counter exists exactly for the count goal."""
    templates = _missions()["templates"]
    assert {t["goal"] for t in templates} == GOALS
    for t in templates:
        assert (t["goal"] == "count") == ("counter" in t), t["id"]
        assert "steps" in t or "progress" in t, t["id"]
    ids = [t["id"] for t in templates]
    assert len(ids) == len(set(ids))


def test_placeholders_are_known() -> None:
    """Texts only use {cible}, {lieu} and {n}; {lieu} only for buildings."""
    for template in _missions()["templates"]:
        for key in ("title", "objective"):
            text = template[key]
            stripped = (
                text.replace("{cible}", "").replace("{lieu}", "").replace("{n}", "")
            )
            assert "{" not in stripped and "}" not in stripped, (template["id"], text)
            if "{lieu}" in text:
                assert template["target"] == "buildable", template["id"]
