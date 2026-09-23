"""Validates the Codex (H2): schema, links, aliases, entities."""

import json
from pathlib import Path

from cent_ans_tools.codex import links_in, validate_codex

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
