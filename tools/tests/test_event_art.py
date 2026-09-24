"""Tests for event miniature prompts, planning and crop (no network)."""

import io
import json
from pathlib import Path

from PIL import Image

from cent_ans_tools import event_art


def _write(directory: Path, name: str, payload: dict) -> None:
    directory.mkdir(parents=True, exist_ok=True)
    (directory / f"{name}.json").write_text(json.dumps(payload), encoding="utf-8")


def _data(tmp_path: Path) -> Path:
    data_dir = tmp_path / "data"
    _write(
        data_dir / "provinces",
        "prov_x",
        {"id": "prov_x", "name": {"display": "Ponthieu"}},
    )
    _write(
        data_dir / "events",
        "evt_a",
        {
            "id": "evt_a",
            "title": "Crécy",
            "text": "Les archers tiennent la colline.",
            "kind": "historical",
            "historical_date": {"value": "1346-08-26"},
            "scope": {"type": "province", "province": "prov_x"},
        },
    )
    _write(
        data_dir / "events",
        "evt_b",
        {"id": "evt_b", "title": "Foire", "text": "Marchands.", "kind": "random"},
    )
    return data_dir


def test_prompt_uses_event_data(tmp_path: Path) -> None:
    """The prompt carries title, place, year and text."""
    jobs = event_art.plan(_data(tmp_path), tmp_path / "out")
    prompt = jobs[0].prompt
    assert "Crécy" in prompt
    assert "Ponthieu, 1346" in prompt
    assert "Les archers tiennent la colline." in prompt
    assert "typical scene" in jobs[1].prompt


def test_plan_is_idempotent(tmp_path: Path) -> None:
    """Existing miniatures are skipped; limit caps the batch."""
    data_dir = _data(tmp_path)
    out_dir = tmp_path / "out"
    out_dir.mkdir()
    (out_dir / "evt_a.jpg").write_bytes(b"x")
    assert [job.character_id for job in event_art.plan(data_dir, out_dir)] == ["evt_b"]
    assert len(event_art.plan(data_dir, tmp_path / "empty", limit=1)) == 1


def test_miniature_crop_is_wide_jpeg() -> None:
    """A square image becomes a 16:9 JPEG band."""
    buffer = io.BytesIO()
    Image.new("RGB", (1024, 1024), (10, 20, 200)).save(buffer, "PNG")
    image = Image.open(io.BytesIO(event_art.to_miniature_jpg(buffer.getvalue())))
    assert image.format == "JPEG"
    assert image.size == (event_art.ART_WIDTH, event_art.ART_HEIGHT)
