"""Tests of the free local 2D stages of the GA3 chains (no mflux, no rembg model, no fal)."""

import importlib
import sys
from pathlib import Path

import pytest
from PIL import Image

sys.path.insert(0, str(Path(__file__).parent.parent / "experiments"))
ga3_local = importlib.import_module("ga3_local")
figure = importlib.import_module("ga3_fal_figure")


def test_local_prompt_is_descriptive_and_white():
    """The edit instruction is dropped; views, A-pose, empty hands, white ground, keys stay."""
    prompt = ga3_local.local_sheet_prompt(figure.UNITS["longbowman"])
    assert "Edit this" not in prompt
    assert "pure white" in prompt and "mid-grey" not in prompt
    for needle in ("front view", "back view", "A-pose", "empty", "green", "blue"):
        assert needle in prompt


def test_backend_choice():
    """Known backends pass, others are refused."""
    assert ga3_local.check_backend("local") == "local"
    with pytest.raises(ValueError):
        ga3_local.check_backend("cloud")


def test_render_local_calls_once_and_caches(tmp_path, monkeypatch):
    """The sheet comes from render_image with the reference and strength; a file is kept."""
    from cent_ans_tools import local_art

    calls = []

    def fake(prompt, **kwargs):
        calls.append(kwargs)
        return b"PNGDATA"

    monkeypatch.setattr(local_art, "render_image", fake)
    reference = tmp_path / "ref.png"
    reference.write_bytes(b"ref")
    out = tmp_path / "sheet.png"
    ga3_local.render_local(
        "p", out, reference=reference, aspect_ratio="3:2", seed=3, strength=0.5
    )
    ga3_local.render_local("p", out, reference=reference)
    assert len(calls) == 1
    assert calls[0] == {
        "aspect_ratio": "3:2",
        "reference": b"ref",
        "seed": 3,
        "strength": 0.5,
    }
    assert out.read_bytes() == b"PNGDATA"


def test_cut_local_uses_remover_and_caches(tmp_path):
    """The cut-out is RGBA, written once; an existing file is never redone."""
    source = tmp_path / "sheet.png"
    Image.new("RGB", (8, 8), "white").save(source)
    seen = []

    def remover(image):
        seen.append(image.size)
        return Image.new("RGBA", image.size, (0, 0, 0, 0))

    out = tmp_path / "sheet_cut.png"
    ga3_local.cut_local(source, out, remover)
    ga3_local.cut_local(source, out, remover)
    assert seen == [(8, 8)]
    assert Image.open(out).mode == "RGBA"
