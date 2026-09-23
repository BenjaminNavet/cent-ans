"""Tests of the icon pipeline (F2): catalogue coverage, normalisation, idempotent build."""

import json
from pathlib import Path

import pytest

from cent_ans_tools import icons, icons_catalog

SAMPLE = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 512 512">'
    '<path d="M0 0h512v512H0z"/><path fill="#fff" d="M10 10h100v100H10z"/></svg>'
)


def fake_fetcher(calls: list[str]):
    """Fetcher returning SAMPLE and recording the requested sources."""

    def fetch(source: str) -> str:
        calls.append(source)
        return SAMPLE

    return fetch


def test_every_data_id_has_an_icon():
    """Units, buildings, resources, technologies, skill branches, trait categories."""
    assert icons.missing_icons() == []


def test_catalogue_authors_and_fallbacks_are_known():
    """Every source has a credited author and every category a fallback."""
    for icon_id, (source, category) in icons_catalog.ICONS.items():
        assert source.count("/") == 1, icon_id
        assert source.split("/")[0] in icons_catalog.AUTHORS, source
        assert category in icons_catalog.FALLBACKS, icon_id
    for fallback in icons_catalog.FALLBACKS.values():
        assert fallback in icons_catalog.ICONS


def test_normalize_removes_background_and_tints():
    """Black square removed, white shapes painted in sepia ink, 64 px."""
    svg = icons.normalize_svg(SAMPLE)
    assert "M0 0h512v512H0z" not in svg
    assert 'fill="#fff"' not in svg
    assert f'fill="{icons.INK}"' in svg
    assert 'width="64"' in svg and 'height="64"' in svg
    assert 'viewBox="0 0 512 512"' in svg


def test_normalize_refuses_entities():
    """A DTD is refused before parsing."""
    with pytest.raises(ValueError):
        icons.normalize_svg('<!DOCTYPE svg [<!ENTITY a "b">]><svg/>')


def test_build_is_idempotent_and_cached(tmp_path: Path):
    """One download per source, then nothing rewritten, offline OK."""
    calls: list[str] = []
    out_dir = tmp_path / "icons"
    cache = tmp_path / "cache"
    rows, written = icons.build(out_dir, cache, fetch=fake_fetcher(calls))
    sources = {row.source for row in rows}
    assert len(calls) == len(sources)
    assert written == 2 * len(sources) + 1  # SVG + .import files + icons.json
    table = json.loads((out_dir / "icons.json").read_text(encoding="utf-8"))
    assert set(table["icons"]) == set(icons_catalog.ICONS)
    assert table["icons"]["unit_knights"]["author"] == "Skoll"
    assert table["license"] == "CC BY 3.0"
    for entry in table["icons"].values():
        assert (out_dir / entry["file"]).exists()
    # Second run: no download, nothing rewritten; offline works from the cache.
    _, written_again = icons.build(out_dir, cache, fetch=fake_fetcher(calls))
    assert written_again == 0
    assert len(calls) == len(sources)
    _, written_offline = icons.build(out_dir, cache, offline=True)
    assert written_offline == 0


def test_offline_without_cache_fails(tmp_path: Path):
    """Offline mode never downloads."""
    with pytest.raises(FileNotFoundError):
        icons.build(tmp_path / "icons", tmp_path / "empty", offline=True)


def test_versioned_table_matches_catalogue():
    """game/assets/icons/icons.json is up to date with the catalogue."""
    table_path = icons.ICONS_DIR / "icons.json"
    table = json.loads(table_path.read_text(encoding="utf-8"))
    assert table == icons.table(icons.entries())
    for entry in table["icons"].values():
        assert (icons.ICONS_DIR / entry["file"]).exists(), entry["file"]


def test_credits_rows_group_by_author():
    """Credits list icon names per author."""
    rows = icons.credits_rows()
    assert "Lorc" in rows and "Delapouite" in rows
    assert "crossed-swords" in rows["Lorc"]


def test_credits_file_lists_every_author_and_icon():
    """CREDITS.md at the repository root attributes every icon (CC BY 3.0)."""
    credits = (icons.REPO_DIR / "CREDITS.md").read_text(encoding="utf-8")
    assert "CC BY 3.0" in credits
    for author, names in icons.credits_rows().items():
        row = f"| {author} | {len(names)} | {', '.join(names)} |"
        assert row in credits, author
