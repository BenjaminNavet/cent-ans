"""Tests for the menu illustration (old parchment map)."""

import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools import menu_art


def _digest(path: Path) -> str:
    return hashlib.sha256(Image.open(path).tobytes()).hexdigest()


def test_render_is_deterministic(tmp_path: Path) -> None:
    """Two small renders are pixel-identical; sidecar gives size and cartouche."""
    first = menu_art.build(out_dir=tmp_path / "a", width=640, height=360)
    second = menu_art.build(out_dir=tmp_path / "b", width=640, height=360)
    assert _digest(first) == _digest(second)
    image = Image.open(first)
    assert image.size == (640, 360)
    assert image.mode == "RGB"
    sidecar = json.loads((tmp_path / "a" / menu_art.SIDECAR_NAME).read_text())
    assert sidecar["size"] == [640, 360]
    x0, y0, x1, y1 = sidecar["cartouche"]
    assert 0 <= x0 < x1 <= 640 and 0 <= y0 < y1 <= 360


def test_render_is_not_flat(tmp_path: Path) -> None:
    """Parchment, ink and sea give a textured, warm image."""
    path = menu_art.build(out_dir=tmp_path, width=480, height=270)
    pixels = np.asarray(Image.open(path), dtype=np.float32)
    assert pixels.std() > 12.0
    red, blue = pixels[..., 0].mean(), pixels[..., 2].mean()
    assert red > blue  # sepia, not grey


def test_faction_capitals_have_names() -> None:
    """Capitals come from data/factions (Paris and London at least)."""
    names = {name for name, _position in menu_art.faction_capitals()}
    assert "Paris" in names
    assert any(name.startswith("Londres") for name in names)
