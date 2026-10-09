"""Validates the Codex (H2): schema, links, aliases, entities."""

import json
from pathlib import Path

from cent_ans_tools.codex import decor_keys, escapes_in, links_in, validate_codex

DATA = Path(__file__).resolve().parents[2] / "data"


def test_real_codex_is_valid() -> None:
    """The seed entries are valid, linked together and point to real entities."""
    report = validate_codex(DATA)
    assert not report.errors, "\n".join(report.errors)
    assert len(report.entries) >= 20
    assert report.link_count >= 40


def test_links_parsing() -> None:
    """Both link forms are recognised."""
    assert links_in("Voir [[cdx_crecy]] et [[cdx_poitiers|Poitiers]].") == [
        "cdx_crecy",
        "cdx_poitiers",
    ]


def _write(path: Path, content: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(content, ensure_ascii=False), encoding="utf-8")


def test_detects_broken_data(tmp_path: Path) -> None:
    """Unresolved links, duplicate aliases, bad ids and unknown entities are reported."""
    for schema in (DATA / "schemas").glob("*.schema.json"):
        _write(
            tmp_path / "schemas" / schema.name,
            json.loads(schema.read_text(encoding="utf-8")),
        )
    base = {"category": "lieu", "summary": "s", "body": "b", "sources": ["x"]}
    _write(
        tmp_path / "codex" / "cdx_a.json",
        base
        | {
            "id": "cdx_a",
            "title": "A",
            "aliases": ["Même"],
            "body": "[[cdx_nope]] [[cdx_later]]",
        },
    )
    _write(
        tmp_path / "codex" / "cdx_b.json",
        base
        | {"id": "cdx_b", "title": "B", "aliases": ["même"], "entity": "chr_ghost"},
    )
    _write(tmp_path / "codex" / "cdx_c.json", base | {"id": "cdx_other", "title": "C"})
    _write(
        tmp_path / "events" / "evt_x.json", {"id": "evt_x", "text": "[[cdx_missing|x]]"}
    )
    (tmp_path / "codex" / "_todo.md").write_text(
        "- `cdx_later` — plus tard\n", encoding="utf-8"
    )
    errors = "\n".join(validate_codex(tmp_path).errors)
    assert "[[cdx_nope]]" in errors
    assert "cdx_later" not in errors
    assert "shared by cdx_a and cdx_b" in errors
    assert "chr_ghost" in errors
    assert "differs from file name" in errors
    assert "evt_x.json.text: unresolved link [[cdx_missing]]" in errors


def test_escapes_are_not_links() -> None:
    """`[[!…]]` escapes are plain text, not links (B8)."""
    text = "[[!Louis de Poitiers]] et [[cdx_poitiers|Poitiers]]"
    assert links_in(text) == ["cdx_poitiers"]
    assert escapes_in(text) == ["Louis de Poitiers"]


def test_exclude_contexts_must_contain_an_alias(tmp_path: Path) -> None:
    """Each `exclude_contexts` expression contains the title or an alias (B8)."""
    for schema in (DATA / "schemas").glob("*.schema.json"):
        _write(
            tmp_path / "schemas" / schema.name,
            json.loads(schema.read_text(encoding="utf-8")),
        )
    base = {"category": "bataille", "summary": "s", "body": "b", "sources": ["x"]}
    _write(
        tmp_path / "codex" / "cdx_p.json",
        base
        | {
            "id": "cdx_p",
            "title": "Bataille de Poitiers",
            "aliases": ["Poitiers"],
            "exclude_contexts": ["Louis de POITIERS", "Jean de Gand"],
        },
    )
    _write(
        tmp_path / "events" / "evt_x.json",
        {"id": "evt_x", "text": "[[!Louis de Poitiers]] [[!]]"},
    )
    errors = "\n".join(validate_codex(tmp_path).errors)
    assert "'Jean de Gand' contains no alias" in errors
    assert "Louis de POITIERS" not in errors
    assert "empty escape" in errors
    assert "unresolved link" not in errors


def test_codex_bundle_is_up_to_date() -> None:
    """data/codex_bundle.json (read by the game) equals a fresh build from the entry files."""
    from cent_ans_tools.codex import BUNDLE_NAME, build_bundle, bundle_text

    committed = (DATA / BUNDLE_NAME).read_text(encoding="utf-8")
    assert committed == bundle_text(build_bundle(DATA)), (
        "stale bundle: run `uv run --project tools cent-ans codex-bundle`"
    )


def test_codex_bundle_matches_schema() -> None:
    """The bundle is valid against codex_bundle.schema.json (entries against codex.schema.json)."""
    from conftest import assert_matches_schema

    assert_matches_schema("codex_bundle.json", "codex_bundle.schema.json")


def test_decor_keys_cover_every_rendered_kind() -> None:
    """NA: the decor keys list trees, battle trees, fauna, birds and rocks of the real data."""
    keys = decor_keys(DATA)
    for key in (
        "tree:oak",
        "battle_tree:ash",
        "fauna:animal_wolf_grey",
        "bird:crane",
        "rock:cliff_limestone",
    ):
        assert key in keys
