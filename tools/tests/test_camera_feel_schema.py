"""Validates data/ui/camera_feel.json (lot PO5: camera inertia, smoothed zoom, focus glide)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_camera_feel_matches_schema() -> None:
    """The camera feel settings match their schema."""
    schema = _load("schemas/camera_feel.schema.json")
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(_load("ui/camera_feel.json"))
    )
    assert not errors, [error.message for error in errors]


def test_reduce_motion_is_never_slower() -> None:
    """Reduced motion never lengthens a glide nor softens the zoom damping."""
    feel = _load("ui/camera_feel.json")
    reduced = feel["reduce_motion"]
    for block in (feel["campaign"], feel["battle"]):
        if "focus_glide_s" in reduced:
            assert reduced["focus_glide_s"] <= block["focus_glide_s"]
        if "zoom_damping" in reduced:
            assert reduced["zoom_damping"] >= block["zoom_damping"]
