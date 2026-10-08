"""Génère docs/decisions/INDEX.md (numéro | titre | statut) depuis les ADR."""

import re
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DECISIONS = ROOT / "docs" / "decisions"


def main() -> None:
    """Écrit l'index ; les numéros en double reçoivent le suffixe « bis »."""
    by_number = defaultdict(list)
    for path in sorted(DECISIONS.glob("[0-9][0-9][0-9][0-9]-*.md")):
        by_number[path.name[:4]].append(path)
    rows = []
    for number, paths in sorted(by_number.items()):
        for rank, path in enumerate(paths):
            text = path.read_text(encoding="utf-8")
            title_match = re.search(r"^# (.+)$", text, re.M)
            title = title_match.group(1) if title_match else path.stem
            title = re.sub(r"^(ADR\s+)?\d+\s*[—:-]\s*", "", title).strip()
            status_match = re.search(r"[Ss]tatut\**\s*:\**\s*([^\n]+)", text)
            status = status_match.group(1).strip(" *") if status_match else "n/d"
            status = re.split(r"[.;(]| le \d", status)[0].strip() or "n/d"
            status = status.replace("|", "/")
            label = number + (" bis" * rank)
            title = title.replace("|", "/")
            rows.append(f"| {label} | [{title}]({path.name}) | {status} |")
    lines = [
        "# Index des décisions d'architecture",
        "",
        "Généré par `tools/adr_index.py` (ne pas éditer à la main). « bis » : numéro attribué deux fois, fichiers non renommés.",
        "Prochain numéro libre : 0193.",
        "",
        "| N° | Titre | Statut |",
        "|---|---|---|",
        *rows,
        "",
    ]
    (DECISIONS / "INDEX.md").write_text("\n".join(lines), encoding="utf-8")
    print(len(rows), "ADR")


if __name__ == "__main__":
    main()
