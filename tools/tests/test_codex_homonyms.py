"""Homonym audit of the Codex auto-links (B8)."""

from pathlib import Path

from cent_ans_tools.codex_homonyms import AutoLinker, audit, suspicion

DATA = Path(__file__).resolve().parents[2] / "data"

ENTRIES = {
    "cdx_poitiers": {
        "title": "Bataille de Poitiers",
        "category": "bataille",
        "aliases": ["Poitiers"],
        "exclude_contexts": ["Louis de Poitiers"],
    },
    "cdx_philippe_vi": {
        "title": "Philippe VI",
        "category": "personnage",
        "aliases": [],
    },
    "cdx_philippe": {"title": "Philippe", "category": "personnage"},
    "cdx_artois": {"title": "Artois", "category": "lieu"},
    "cdx_omer": {"title": "Omer", "category": "lieu"},
}


def _links(text: str) -> list[tuple[str, str]]:
    linker = AutoLinker(ENTRIES)
    return [
        (entry_id, segment[start:end])
        for entry_id, start, end, segment in linker.links(text)
    ]


def test_excluded_context_is_not_linked() -> None:
    """« Louis de Poitiers » is skipped, a later « à Poitiers » is linked."""
    assert _links("Louis de Poitiers meurt.") == []
    assert _links("Louis de Poitiers meurt ; bataille à Poitiers.") == [
        ("cdx_poitiers", "Poitiers")
    ]


def test_longest_alias_wins() -> None:
    """« Philippe VI » links the king, not the shorter « Philippe »."""
    assert _links("Philippe VI règne.") == [("cdx_philippe_vi", "Philippe VI")]


def test_word_boundaries() -> None:
    """The apostrophe separates words, the hyphen between two words joins them."""
    assert _links("Robert d'Artois") == [("cdx_artois", "Artois")]
    assert _links("Saint-Omer et Omer-la-Ville") == []
    assert _links("Omer - fin") == [("cdx_omer", "Omer")]


def test_escapes_and_explicit_links_are_skipped() -> None:
    """`[[!…]]` is never auto-linked; an explicitly linked entry is not linked twice."""
    assert _links("[[!Poitiers]] puis Artois") == [("cdx_artois", "Artois")]
    assert _links("[[cdx_artois|l'Artois]] et Artois") == []


def test_suspicion_heuristics() -> None:
    """Names, titles of non-territorial entries and reign numbers are suspect."""
    names = {"Louis"}
    text = "Louis de Poitiers"
    assert suspicion(text, 9, 17, "bataille", names) == "nom « X de Alias »"
    text = "Le comte de Poitiers"
    assert suspicion(text, 12, 20, "bataille", names) == "titre « … de Alias »"
    assert suspicion("Le duc de Bourgogne", 10, 19, "lieu", names) == ""
    assert suspicion("Vignoble de Bordeaux", 12, 20, "lieu", names) == ""
    assert suspicion("bois de chêne", 0, 4, "economie", names) == ""


def test_real_data_known_homonyms_are_fixed() -> None:
    """The false links fixed by B8 do not come back."""
    fixed = {
        ("cdx_poitiers", "Louis de Poitiers"),
        ("cdx_orleans", "Philippe d'Orléans"),
        ("cdx_flandre_laine", "bâtard de Flandre"),
        ("cdx_gabelle", "Guigone de Salins"),
        ("cdx_cogue", "Christophe II"),
    }
    for finding in audit(DATA):
        for entry_id, context in fixed:
            assert not (finding.entry_id == entry_id and context in finding.snippet), (
                finding
            )
