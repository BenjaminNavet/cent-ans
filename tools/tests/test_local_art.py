"""Tests of the free local mflux backend (no model run: fake runner)."""

import io
from decimal import Decimal
from pathlib import Path

import pytest
from PIL import Image

from cent_ans_tools import entry_art, local_art, openrouter, portraits
from cent_ans_tools.portraits import PortraitJob


def _fake_run(command: list[str]) -> None:
    output = Path(command[command.index("--output") + 1])
    width = int(command[command.index("--width") + 1])
    height = int(command[command.index("--height") + 1])
    Image.new("RGB", (width, height), "navy").save(output)


@pytest.fixture
def fake_mflux(monkeypatch):
    """Replace the mflux subprocess by a fake that records its command lines."""
    commands: list[list[str]] = []

    def run(command: list[str]) -> None:
        commands.append(command)
        _fake_run(command)

    monkeypatch.setattr(local_art, "_run", run)
    return commands


def test_render_image_honours_aspect_and_stable_seed(fake_mflux):
    """The aspect ratio sets the size; the same prompt gives the same seed."""
    image = local_art.render_image("a realm", aspect_ratio="3:4")
    assert Image.open(io.BytesIO(image)).size == (768, 1024)
    local_art.render_image("a realm", aspect_ratio="3:4")
    seeds = [c[c.index("--seed") + 1] for c in fake_mflux]
    assert seeds[0] == seeds[1]


def test_render_image_passes_reference(fake_mflux):
    """A reference image becomes the img2img start."""
    buffer = io.BytesIO()
    Image.new("RGB", (8, 8)).save(buffer, "PNG")
    local_art.render_image("p", reference=buffer.getvalue())
    assert "--image-path" in fake_mflux[0]


def test_unknown_aspect_is_refused():
    """An aspect ratio without a local size fails loudly."""
    with pytest.raises(ValueError):
        local_art.size_for("5:1")


def test_openrouter_routes_local_model_for_free(fake_mflux):
    """``request_image`` with the local model renders here, at no cost."""
    image, cost = openrouter.request_image(
        local_art.MODEL_ID, "p", image_config={"aspect_ratio": "1:1"}
    )
    assert cost == Decimal("0")
    assert Image.open(io.BytesIO(image)).size == (1024, 1024)
    assert openrouter.estimate_price(local_art.MODEL_ID) == Decimal("0")


def test_batch_with_local_model_writes_without_budget_row(
    fake_mflux, tmp_path, budget_file
):
    """A shared batch on the local model writes miniatures and no ledger row."""
    before = budget_file.read_text(encoding="utf-8")
    job = PortraitJob("fac_test", "A realm", tmp_path / "fac_test.jpg")
    result = portraits.generate(
        [job],
        local_art.MODEL_ID,
        budget_path=budget_file,
        convert=entry_art.convert,
        image_config={"aspect_ratio": "16:9"},
    )
    assert result.written == [job.out_path]
    assert result.actual == Decimal("0")
    assert budget_file.read_text(encoding="utf-8") == before
    image = Image.open(io.BytesIO(job.out_path.read_bytes()))
    assert image.size == (entry_art.ART_WIDTH, entry_art.ART_HEIGHT)


def test_strength_overrides_reference_strength(fake_mflux):
    """``strength`` replaces the default img2img strength; omitted keeps it."""
    buffer = io.BytesIO()
    Image.new("RGB", (8, 8)).save(buffer, "PNG")
    local_art.render_image("p", reference=buffer.getvalue(), strength=0.7)
    local_art.render_image("p", reference=buffer.getvalue())
    values = [c[c.index("--image-strength") + 1] for c in fake_mflux]
    assert values == ["0.7", str(local_art.REFERENCE_STRENGTH)]
