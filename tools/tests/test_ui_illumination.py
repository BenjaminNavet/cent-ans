"""DN ui-kit: the generated illumination kit covers every widget family."""

import json
from pathlib import Path

from cent_ans_tools import ui_illumination

NEW_PIECES = [
    *(f"button_primary_{s}" for s in ("normal", "hover", "pressed", "disabled")),
    "tab_ornate_selected",
    "tab_ornate_unselected",
    "progress_frame",
    "progress_fill",
    "slider_grabber",
    "slider_grabber_hover",
    "separator_h",
    "inset_ornate",
    "initial_frame",
]


def test_build_is_deterministic_and_complete(tmp_path: Path) -> None:
    """Building twice yields identical PNGs, and the sidecar lists every new piece."""
    first = tmp_path / "a"
    second = tmp_path / "b"
    ui_illumination.build(first)
    ui_illumination.build(second)
    sidecar = json.loads((first / ui_illumination.SIDECAR_NAME).read_text())
    for name in NEW_PIECES:
        assert name in sidecar
        assert (first / f"{name}.png").read_bytes() == (
            second / f"{name}.png"
        ).read_bytes()


def test_historical_pieces_unchanged(tmp_path: Path) -> None:
    """Appending pieces must not repaint the historical textures (those without an nb override)."""
    ui_illumination.build(tmp_path)
    for name in ("button_normal", "button_hover", "tab_selected", "inset"):
        committed = ui_illumination.OUTPUT_DIR / f"{name}.png"
        assert (tmp_path / f"{name}.png").read_bytes() == committed.read_bytes()


def test_contact_sheet(tmp_path: Path) -> None:
    """The review board renders without a GPU."""
    out = ui_illumination.contact_sheet(tmp_path / "sheet.png")
    assert out.exists() and out.stat().st_size > 1000
