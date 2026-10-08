"""Validates data/ui/camera_feel.json (lot PO5: camera inertia, smoothed zoom, focus glide)."""

import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_reduce_motion_is_never_slower() -> None:
    """Reduced motion never lengthens a glide nor softens the zoom damping."""
    feel = _load("ui/camera_feel.json")
    reduced = feel["reduce_motion"]
    for block in (feel["campaign"], feel["battle"]):
        if "focus_glide_s" in reduced:
            assert reduced["focus_glide_s"] <= block["focus_glide_s"]
        if "zoom_damping" in reduced:
            assert reduced["zoom_damping"] >= block["zoom_damping"]
