"""Normalise les apostrophes visibles en ’ (typographique) dans `data/` et les textes de l'UI.

Usage : `uv run --project tools python tools/typo_apostrophes.py [--check]`.
- JSON de `data/` : seules les valeurs de clés d'affichage (`INCLUDED_KEYS`) sont touchées, jamais
  les identifiants, alias, sources, blasons, invites de génération ni la voix (clé de cache audio).
- Le client Bevy (`client/src`) n'est pas scanné : ses textes viennent de `data/` (ADR 0366).
`--check` : n'écrit rien, sort avec le code 1 s'il reste des apostrophes droites à convertir.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
INCLUDED_KEYS = {
    "text",
    "title",
    "body",
    "summary",
    "gameplay",
    "description",
    "intro",
    "tagline",
    "strengths",
    "weaknesses",
    "label",
    "objective",
    "display",
    "description_fr",
    "journal",
    "journal_failure",
    "journal_assault",
    "fr",
    "local",
}
EXCLUDED_DIRS = {
    "schemas",
    "art",
    "voice",
    "speeches",
    "heraldry",
    "landmarks_v2",
    "landmarks",
}
WORD_APOSTROPHE = re.compile(r"(?<=\w)'(?=[\w\[])")


def _collect(node, key: str, found: set[str]) -> None:
    if isinstance(node, dict):
        for child_key, child in node.items():
            _collect(child, child_key, found)
    elif isinstance(node, list):
        for child in node:
            _collect(child, key, found)
    elif (
        isinstance(node, str)
        and (key in INCLUDED_KEYS or key.startswith("fac_"))
        and WORD_APOSTROPHE.search(node)
    ):
        found.add(node)


def convert_json_file(path: Path, write: bool) -> int:
    """Convertit les valeurs d'affichage d'un JSON ; renvoie le nombre de remplacements."""
    text = path.read_text(encoding="utf-8")
    try:
        data = json.loads(text)
    except ValueError:
        return 0
    found: set[str] = set()
    _collect(data, "", found)
    changed = 0
    for old in sorted(found, key=len, reverse=True):
        encoded = json.dumps(old, ensure_ascii=False)
        new_encoded = WORD_APOSTROPHE.sub("’", encoded)
        count = text.count(encoded)
        if count:
            text = text.replace(encoded, new_encoded)
            changed += count
    if changed and write:
        path.write_text(text, encoding="utf-8")
    return changed


def main() -> int:
    """Point d'entrée en ligne de commande."""
    write = "--check" not in sys.argv
    total = 0
    for path in sorted((ROOT / "data").rglob("*.json")):
        relative = path.relative_to(ROOT / "data")
        if relative.parts[0] in EXCLUDED_DIRS or relative.name == "codex_bundle.json":
            continue
        total += convert_json_file(path, write)
    print(f"{total} apostrophe(s) {'à convertir' if not write else 'converties'}")
    return 1 if (total and not write) else 0


if __name__ == "__main__":
    sys.exit(main())
