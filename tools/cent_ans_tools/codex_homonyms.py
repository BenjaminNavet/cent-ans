"""Homonym audit of the Codex auto-links (B8).

`CodexText.format(text, true)` (Godot) links the first occurrence of every Codex alias found in
a displayed text. This module replays that auto-link in Python (same alias regex, same word
boundaries, `exclude_contexts` and `[[!…]]` escapes honoured) over the texts of `data/`, and
reports the auto-links whose alias sits inside a longer proper-noun sequence, which usually
designates a homonym: « Louis de Poitiers » is not the battle of Poitiers, « Charles de Blois »
is not the town of Blois, « Philippe VI » is not a generic « Philippe ».

Run: `uv run --project tools python -m cent_ans_tools.codex_homonyms [data_dir]`.
"""

import json
import re
import sys
from collections.abc import Iterator
from dataclasses import dataclass
from pathlib import Path

from cent_ans_tools.codex import ESCAPE_PREFIX, entry_forms, iter_strings

## Directories whose texts are shown through `CodexText.format(…, true)` (tooltips, character
## sheets, chronicle, battle orders) plus the Codex itself (bubbles, window).
AUDITED_DIRECTORIES = (
    "unit_types",
    "buildings",
    "technologies",
    "resources",
    "edicts",
    "events",
    "characters",
    "battle_orders",
    "diets",
    "chivalric_orders",
    "traits",
    "skills",
    "factions",
    "religions",
    "naval",
    "codex",
)
## Codex fields displayed to the player (ids, sources and aliases are not).
CODEX_TEXT_FIELDS = ("summary", "body", "gameplay", "anachronism")
## Keys of game data never shown as prose (bibliography, identifiers).
SKIPPED_KEYS = {"sources", "id", "local", "local_language"}
## Entries naming a territory or a house: « duc de Bourgogne », « maison de Valois » may link
## them. For any other category (battle, siege, economy...) the title is a homonym.
TERRITORY_CATEGORIES = {"lieu", "dynastie"}

## `[[…]]` links and escapes, and BBCode tags: the auto-link never looks inside them.
PROTECTED_PATTERN = re.compile(r"\[\[[^\]]*\]\]|\[[^\[\]]*\]")
UPPER = "A-ZÀ-ÖØ-Ý"
PARTICLE = r"(?:de\s+la\s+|de\s+l['’]|de\s+|du\s+|des\s+|d['’])"
## « Louis de Poitiers », « Charles de Blois », « Jean d'Artois » : the word before the
## particle (group 2), and what precedes it (group 1, to tell a sentence start).
NAME_BEFORE = re.compile(
    rf"(^|.{{0,3}})\b([{UPPER}][\w'’-]*)\s+(?:[IVX]+\s+)?{PARTICLE}$"
)
SENTENCE_START = re.compile(r"(^|[.!?:;«»\n]\s*|\(\s*)$")
## « comte de Poitiers », « sire de Coucy », « évêque de Beauvais ».
TITLE_BEFORE = re.compile(
    r"\b(?:comte|comtesse|vicomte|duc|duchesse|sire|seigneur|dame|évêque|archevêque|abbé|"
    r"prince|princesse|bâtard|baron|captal|maréchal|connétable|chevalier|écuyer|"
    rf"maison|lignage|famille|héritier|héritière)s?\s+{PARTICLE}$",
    re.IGNORECASE,
)
## « saint Louis », « Notre-Dame de Paris ».
SAINT_BEFORE = re.compile(r"\b(?:saint|sainte|notre-dame)\s+(?:de\s+)?$", re.IGNORECASE)
## « Philippe VI », « Édouard III ».
ORDINAL_AFTER = re.compile(r"^\s+(?:[IVX]+|Ier|Ire|1er)\b")


@dataclass(frozen=True)
class AutoLink:
    """One auto-link the game would draw, with the suspicion it raises (if any)."""

    where: str
    entry_id: str
    alias: str
    snippet: str
    reason: str


def alias_pattern(aliases: list[str]) -> re.Pattern[str]:
    """Regex equivalent to `CodexStore.alias_regex` (longest first, hyphen-aware words)."""
    ordered = sorted(aliases, key=len, reverse=True)
    alternation = "|".join(re.escape(alias) for alias in ordered)
    return re.compile(
        rf"(?<![\w])(?<!\w-)({alternation})(?!\w|-\w)", re.IGNORECASE | re.UNICODE
    )


def load_entries(data_dir: Path) -> dict[str, dict]:
    """Codex entries by id, in file-name order (the order `CodexStore` reads them)."""
    entries = {}
    for path in sorted((data_dir / "codex").glob("cdx_*.json")):
        entry = json.loads(path.read_text(encoding="utf-8"))
        entries[str(entry.get("id", path.stem))] = entry
    return entries


class AutoLinker:
    """Replays `CodexText._auto_link` on plain data texts."""

    def __init__(self, entries: dict[str, dict]) -> None:
        """Indexes the aliases and excluded contexts of `entries`."""
        self.alias_to_id: dict[str, str] = {}
        self.exclusions: dict[str, list[str]] = {}
        for entry_id, entry in entries.items():
            for form in entry_forms(entry):
                self.alias_to_id.setdefault(form.lower(), entry_id)
            contexts = [
                str(context).strip().lower()
                for context in entry.get("exclude_contexts", [])
            ]
            if contexts:
                self.exclusions[entry_id] = contexts
        self.pattern = alias_pattern(list(self.alias_to_id))

    def is_excluded(self, entry_id: str, text: str, start: int, end: int) -> bool:
        """Same test as `CodexStore.is_excluded`."""
        alias = text[start:end].lower()
        for context in self.exclusions.get(entry_id, []):
            offset = context.find(alias)
            while offset >= 0:
                begin = start - offset
                if begin >= 0 and text[begin : begin + len(context)].lower() == context:
                    return True
                offset = context.find(alias, offset + 1)
        return False

    def links(self, text: str) -> Iterator[tuple[str, int, int, str]]:
        """Yields `(entry_id, start, end, segment)` for each auto-link of `text`."""
        linked = {
            match.group(0)[2:-2].split("|")[0].strip()
            for match in PROTECTED_PATTERN.finditer(text)
            if match.group(0).startswith("[[")
            and not match.group(0).startswith("[[" + ESCAPE_PREFIX)
        }
        cursor = 0
        spans = [*PROTECTED_PATTERN.finditer(text), None]
        for span in spans:
            end = span.start() if span is not None else len(text)
            segment = text[cursor:end]
            for match in self.pattern.finditer(segment):
                entry_id = self.alias_to_id.get(match.group(1).lower(), "")
                if not entry_id or entry_id in linked:
                    continue
                if self.is_excluded(entry_id, segment, match.start(1), match.end(1)):
                    continue
                linked.add(entry_id)
                yield entry_id, match.start(1), match.end(1), segment
            if span is None:
                break
            cursor = span.end()


def given_names(data_dir: Path) -> set[str]:
    """First names of `data/names/*.json` and of the historical characters."""
    names: set[str] = set()
    for path in (data_dir / "names").glob("*.json"):
        content = json.loads(path.read_text(encoding="utf-8"))
        for key in ("male_first_names", "female_first_names"):
            names |= {str(name) for name in content.get(key, [])}
    for path in (data_dir / "characters").glob("*.json"):
        display = json.loads(path.read_text(encoding="utf-8")).get("name", {})
        if isinstance(display, dict) and display.get("display"):
            names.add(str(display["display"]).split()[0])
    return names


def suspicion(
    segment: str, start: int, end: int, category: str, names: set[str]
) -> str:
    """Why an auto-link at `segment[start:end]` looks like a homonym, or empty.

    Only capitalised occurrences are proper nouns; « bois de chêne » is not a homonym.
    """
    alias = segment[start:end]
    if not alias[:1].isupper():
        return ""
    before = segment[max(0, start - 60) : start]
    after = segment[end : end + 40]
    name = NAME_BEFORE.search(before)
    if name:
        word = name.group(2)
        prefix = segment[: max(0, start - 60) + name.start(2)]
        if word in names or not SENTENCE_START.search(prefix):
            return "nom « X de Alias »"
    if category not in TERRITORY_CATEGORIES and TITLE_BEFORE.search(before):
        return "titre « … de Alias »"
    if SAINT_BEFORE.search(before):
        return "saint / Notre-Dame"
    if ORDINAL_AFTER.search(after):
        return "numéro de règne « Alias VI »"
    return ""


def iter_texts(data_dir: Path) -> Iterator[tuple[str, str]]:
    """Yields `(location, text)` for the displayed texts of the audited directories."""
    for directory in AUDITED_DIRECTORIES:
        for path in sorted((data_dir / directory).rglob("*.json")):
            try:
                content = json.loads(path.read_text(encoding="utf-8"))
            except json.JSONDecodeError:
                continue
            relative = str(path.relative_to(data_dir))
            if directory == "codex":
                for field in CODEX_TEXT_FIELDS:
                    if isinstance(content.get(field), str):
                        yield f"{relative}.{field}", content[field]
                continue
            for where, text in iter_strings(content, relative):
                keys = set(re.findall(r"\.(\w+)", where.removeprefix(relative)))
                if keys & SKIPPED_KEYS or " " not in text.strip():
                    continue  # bibliography, ids and single words are not prose
                yield where, text


def audit(data_dir: Path) -> list[AutoLink]:
    """Suspicious auto-links of every audited text of `data_dir`."""
    entries = load_entries(data_dir)
    linker = AutoLinker(entries)
    names = given_names(data_dir)
    findings = []
    for where, text in iter_texts(data_dir):
        own_id = Path(where.split(".json")[0]).name if "codex/" in where else ""
        for entry_id, start, end, segment in linker.links(text):
            if entry_id == own_id:
                continue
            category = str(entries[entry_id].get("category", ""))
            reason = suspicion(segment, start, end, category, names)
            if reason:
                snippet = segment[max(0, start - 40) : end + 30].replace("\n", " ")
                findings.append(
                    AutoLink(where, entry_id, segment[start:end], snippet, reason)
                )
    return findings


def main(argv: list[str] | None = None) -> int:
    """Prints the suspicious auto-links; the exit code is always 0 (review list)."""
    args = sys.argv[1:] if argv is None else argv
    data_dir = Path(args[0]) if args else Path(__file__).resolve().parents[2] / "data"
    findings = audit(data_dir)
    for finding in findings:
        print(
            f"{finding.where}: « {finding.alias} » → {finding.entry_id} "
            f"[{finding.reason}] …{finding.snippet}…"
        )
    print(f"{len(findings)} auto-liens suspects")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
