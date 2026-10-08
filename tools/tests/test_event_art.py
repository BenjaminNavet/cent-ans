"""Tests for event miniature prompts, planning and crop (no network)."""

import io
import json
from pathlib import Path

from PIL import Image

from cent_ans_tools import codex_art, event_art


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


def test_entry_art_prompts_and_plan(tmp_path: Path) -> None:
    """Unit, building and technology prompts come from the data; plan is idempotent."""
    from cent_ans_tools import entry_art

    data_dir = tmp_path / "data"
    _write(
        data_dir / "unit_types",
        "unit_x",
        {
            "id": "unit_x",
            "name": {"display": "Archers", "local": "archiers"},
            "equipment": "Arc long.",
            "description": "Tireurs.",
        },
    )
    _write(
        data_dir / "technologies",
        "tech_x",
        {
            "id": "tech_x",
            "name": {"display": "Alambic"},
            "historical_year": {"value": "1351"},
            "description": "Distillation.",
        },
    )
    out_dir = tmp_path / "out"
    jobs = entry_art.plan(data_dir, out_dir)
    assert [job.character_id for job in jobs] == ["unit_x", "tech_x"]
    assert "« Archers » (archiers)" in jobs[0].prompt
    assert "Arc long." in jobs[0].prompt
    assert "around 1351" in jobs[1].prompt
    out_dir.mkdir()
    (out_dir / "unit_x.jpg").write_bytes(b"x")
    assert [job.character_id for job in entry_art.plan(data_dir, out_dir)] == ["tech_x"]


def test_codex_plan_skips_entities_with_art_and_orders_by_category(tmp_path):
    """Codex entries reuse entity art; others are planned by category priority."""
    data_dir = tmp_path / "data"
    entries = {
        "cdx_recette": {"category": "recette", "summary": "Un plat."},
        "cdx_paris": {
            "category": "lieu",
            "era": {"from": "1337", "to": "1453"},
            "summary": "Capitale de [[cdx_charles_v|Charles V]] et du [[cdx_parlement]].",
        },
        "cdx_crecy": {"category": "bataille", "entity": "evt_crecy", "summary": "."},
        "cdx_roi": {"category": "personnage", "summary": "Un roi."},
    }
    for entry_id, entry in entries.items():
        _write(
            data_dir / "codex", entry_id, {"id": entry_id, "title": entry_id, **entry}
        )
    assets_dir = tmp_path / "assets"
    (assets_dir / "events").mkdir(parents=True)
    (assets_dir / "events" / "evt_crecy.jpg").write_bytes(b"x")
    jobs = codex_art.plan(data_dir, assets_dir)
    assert [job.character_id for job in jobs] == ["cdx_paris", "cdx_recette"]
    assert "period 1337-1453" in jobs[0].prompt
    assert "Capitale de Charles V et du cdx_parlement." in jobs[0].prompt
    assert jobs[0].out_path == assets_dir / "illustrations" / "cdx_paris.jpg"


def test_faction_prompt_uses_capital_province_scenery(tmp_path: Path) -> None:
    """A faction prompt describes its capital province and a stable scene."""
    from cent_ans_tools import entry_art

    data_dir = tmp_path / "data"
    _write(
        data_dir / "factions",
        "fac_x",
        {
            "id": "fac_x",
            "name": {"display": "Alanie"},
            "capital": "prov_x",
            "capital_city": "Alagir",
            "description": "Montagnards.",
        },
    )
    _write(
        data_dir / "provinces",
        "prov_x",
        {"terrain": "mountains", "religion": "rel_orthodox", "rivers": ["Terek"]},
    )
    jobs = entry_art.plan(data_dir, tmp_path / "out", categories=("factions",))
    prompt = jobs[0].prompt
    assert "snow-capped mountains" in prompt
    assert "Byzantine-style domed churches" in prompt
    assert "the river Terek" in prompt
    assert (
        prompt
        == entry_art.plan(data_dir, tmp_path / "out", categories=("factions",))[
            0
        ].prompt
    )


def test_miniature_inset_drops_painted_frame() -> None:
    """``inset`` cuts a border painted around the image before the ratio crop."""
    image = Image.new("RGB", (1000, 562), "gold")
    image.paste(Image.new("RGB", (970, 532), "navy"), (15, 15))
    buffer = io.BytesIO()
    image.save(buffer, "PNG")
    out = Image.open(
        io.BytesIO(event_art.to_miniature_jpg(buffer.getvalue(), inset=0.03))
    )
    red, green, blue = out.convert("RGB").getpixel((0, 0))
    assert blue > red and blue > green
