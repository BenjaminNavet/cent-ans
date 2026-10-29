"""One word per notion in player-facing text (audit XP-LISIBLE, M7).

`data/ui/glossary.json` lists a canonical term and its forbidden forms; this test sweeps the text
fields of the data files and the French string literals of the Rust code named by the glossary's
scopes, and fails on a forbidden form.
"""

import json
import re
from pathlib import Path

import pytest
from conftest import assert_matches_schema

ROOT = Path(__file__).resolve().parents[2]
GLOSSARY = json.loads((ROOT / "data" / "ui" / "glossary.json").read_text(encoding="utf-8"))
STRING_LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"')


def _forbidden_pattern() -> re.Pattern[str]:
    forms = [form for term in GLOSSARY["terms"] for form in term["forbidden"]]
    return re.compile(r"\b(" + "|".join(re.escape(f) for f in forms) + r")\b", re.IGNORECASE)


def _allowed(text: str) -> bool:
    return any(phrase in text.lower() for phrase in GLOSSARY.get("allow", []))


def _json_strings(value, key: str, scope: dict, path: str = ""):
    """Yield (path, text) for every string under `value` that the scope retains."""
    if isinstance(value, dict):
        for child_key, child in value.items():
            if child_key in scope.get("skip_keys", []):
                continue
            yield from _json_strings(child, child_key, scope, f"{path}/{child_key}")
    elif isinstance(value, list):
        for index, child in enumerate(value):
            yield from _json_strings(child, key, scope, f"{path}[{index}]")
    elif isinstance(value, str):
        keys = scope.get("keys")
        if keys is None or key in keys:
            yield path, value


def _rust_literals(source: str):
    """Yield (line number, literal) for string literals outside comment lines."""
    for number, line in enumerate(source.splitlines(), 1):
        if line.lstrip().startswith("//"):
            continue
        code = line.split(" //", 1)[0]
        for match in STRING_LITERAL.finditer(code):
            yield number, match.group(1)


def _violations() -> list[str]:
    pattern = _forbidden_pattern()
    found: list[str] = []
    for scope in GLOSSARY["scopes"]:
        for file in sorted(ROOT.glob(scope["files"])):
            relative = file.relative_to(ROOT)
            if scope.get("kind") == "rust":
                texts = (
                    (f"{relative}:{number}", text)
                    for number, text in _rust_literals(file.read_text(encoding="utf-8"))
                )
            else:
                data = json.loads(file.read_text(encoding="utf-8"))
                texts = ((f"{relative}{path}", text) for path, text in _json_strings(data, "", scope))
            for where, text in texts:
                match = pattern.search(text)
                if match and not _allowed(text):
                    found.append(f"{where}: « {match.group(1)} » dans {text[:80]!r}")
    return found


def test_glossary_matches_its_schema() -> None:
    """The glossary follows its schema."""
    assert_matches_schema("ui/glossary.json", "ui_glossary.schema.json")


def test_glossary_scopes_find_files() -> None:
    """Every scope matches at least one file (a typo in a glob would silently disable it)."""
    for scope in GLOSSARY["scopes"]:
        assert list(ROOT.glob(scope["files"])), scope["files"]


def test_no_forbidden_term_in_player_facing_text() -> None:
    """No text read by the player uses a form the glossary forbids."""
    violations = _violations()
    assert not violations, "termes interdits :\n" + "\n".join(violations)


@pytest.mark.parametrize(
    ("text", "hit"),
    [("−10 agitation", True), ("−10 mécontentement", False), ("des colonies", True)],
)
def test_forbidden_pattern_detects_forms(text: str, hit: bool) -> None:
    """The matcher sees whole words only, whatever their case."""
    assert bool(_forbidden_pattern().search(text)) is hit
