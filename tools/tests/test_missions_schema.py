"""Validates data/missions.json against its schema (lot NT3, ADR 0127)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

DATA = Path(__file__).resolve().parents[2] / "data"

KINDS = {
    "take_province",
    "win_battle",
    "construct_building",
    "recruit_units",
    "conclude_treaty",
    "hold_place",
}


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _missions() -> dict:
    return _load(DATA / "missions.json")


def test_missions_match_schema() -> None:
    """The missions file exists and matches its schema."""
    schema = _load(DATA / "schemas" / "missions.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_missions()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_every_kind_has_a_template_with_unique_ids() -> None:
    """The six kinds of the spec are offered; template ids are unique."""
    templates = _missions()["templates"]
    assert {t["kind"] for t in templates} == KINDS
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
                assert template["kind"] == "construct_building", template["id"]
