"""Codex nature plates (bestiary / herbal style) and image-reuse detection."""

import json
from pathlib import Path

from cent_ans_tools import codex_art


def _entry(**fields) -> dict:
    return {"id": "cdx_x", "title": "Le x", "summary": "Texte.", **fields}


def test_nature_prompt_is_a_bestiary_plate_with_english_subject() -> None:
    """A nature entry gets an isolated-subject vellum plate, not a scene."""
    prompt = codex_art.build_prompt(
        _entry(id="cdx_castor", category="animal", title="Le castor")
    )
    assert "bestiary plate showing a European beaver" in prompt
    assert "vellum" in prompt
    assert "A scene" not in prompt


def test_tree_prompt_is_herbal_and_unknown_falls_back_to_title() -> None:
    """Trees use the herbal book; an unmapped entry keeps its title and Latin name."""
    prompt = codex_art.build_prompt(
        _entry(category="arbre", title="Le sorbier", latin="Sorbus")
    )
    assert "herbal plate showing « Le sorbier » (Sorbus)" in prompt
    assert "medieval herbal" in prompt


def test_every_mapped_subject_has_a_codex_entry() -> None:
    """The English subject table only names real nature entries."""
    data_dir = Path(__file__).resolve().parents[2] / "data" / "codex"
    for entry_id in codex_art.NATURE_SUBJECTS:
        entry = json.loads((data_dir / f"{entry_id}.json").read_text(encoding="utf-8"))
        assert entry["category"] in codex_art.NATURE_CATEGORIES


def test_slug_portrait_counts_as_an_image_but_not_an_entity_icon(tmp_path) -> None:
    """The game falls back on portraits/chr_<slug>.png; icons/entity is too small."""
    assets = tmp_path / "assets"
    (assets / "portraits").mkdir(parents=True)
    (assets / "icons" / "entity").mkdir(parents=True)
    (assets / "portraits" / "chr_jean.png").write_bytes(b"x")
    (assets / "icons" / "entity" / "res_wheat.png").write_bytes(b"x")
    assert codex_art.has_entity_image(_entry(id="cdx_jean"), assets)
    wheat = _entry(id="cdx_ble", entity="res_wheat", category="economie")
    assert not codex_art.has_entity_image(wheat, assets)


def test_plan_gives_plants_their_own_plate_even_with_entity_art(tmp_path) -> None:
    """A plant whose entity is a tech scene is still planned as a herbal plate."""
    data_dir = tmp_path / "data" / "codex"
    data_dir.mkdir(parents=True)
    entry = _entry(id="cdx_saule", category="plante", entity="tech_willow_bark")
    (data_dir / "cdx_saule.json").write_text(json.dumps(entry), encoding="utf-8")
    assets = tmp_path / "assets"
    (assets / "illustrations").mkdir(parents=True)
    (assets / "illustrations" / "tech_willow_bark.jpg").write_bytes(b"x")
    jobs = codex_art.plan(tmp_path / "data", assets)
    assert [job.character_id for job in jobs] == ["cdx_saule"]
    assert "willow" in jobs[0].prompt


def test_economy_prompt_is_a_titleless_scene() -> None:
    """Economy plates describe a concrete work scene and never repeat the title."""
    prompt = codex_art.build_prompt(
        _entry(id="cdx_fer", category="economie", title="Le fer")
    )
    assert "blacksmiths" in prompt
    assert "« Le fer »" not in prompt
