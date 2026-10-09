"""Langue de réplique de chaque faction (lot RX audio, ADR 0247).

`CULTURE_LANGUAGE` associe une culture (`data/factions/*.json`, champ `culture`) à la langue de
`data/voice/barks.json` la plus proche parmi celles qui ont des enregistrements. Les cultures sans
langue proche (slave, turcique, arabe, grec, caucasienne, hongroise, baltique, finno-ougrienne,
albanaise) restent sur la langue `default` jusqu'à l'enregistrement de nouvelles voix.

`uv run --project tools python -m cent_ans_tools.barks_languages` complète `faction_language`.
"""

from __future__ import annotations

import json
import re
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
BARKS = REPO / "data" / "voice" / "barks.json"

CULTURE_LANGUAGE = {
    "cul_french": "fr", "cul_breton": "fr", "cul_savoyard": "fr",
    "cul_occitan": "oc", "cul_catalan": "oc", "cul_castilian": "oc", "cul_navarrese": "oc",
    "cul_portuguese": "oc", "cul_tuscan": "oc", "cul_lombard": "oc", "cul_ligurian": "oc",
    "cul_neapolitan": "oc", "cul_venetian": "oc", "cul_sicilian": "oc", "cul_sardinian": "oc",
    "cul_dalmatian": "oc", "cul_romanian": "oc",
    "cul_english": "en", "cul_scottish": "sco", "cul_irish": "cy",
    "cul_german": "nl", "cul_dutch": "nl", "cul_flemish": "nl", "cul_danish": "nl", "cul_swedish": "nl",
}  # fmt: skip


def faction_cultures() -> dict[str, str]:
    """Identifiant de faction → culture, pour les 177 fichiers de `data/factions/`."""
    result = {}
    for path in sorted((REPO / "data" / "factions").glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        result[data["id"]] = data.get("culture", "")
    return result


def complete_faction_language() -> int:
    """Ajoute à `faction_language` les factions absentes dont la culture a une langue proche."""
    text = BARKS.read_text(encoding="utf-8")
    existing = set(json.loads(text)["faction_language"])
    lines = [
        f'    "{faction}": "{CULTURE_LANGUAGE[culture]}",\n'
        for faction, culture in faction_cultures().items()
        if faction not in existing and culture in CULTURE_LANGUAGE
    ]
    # Insère avant l'entrée "default" (dernière du bloc).
    text = re.sub(
        r'(    "default": "fr"\n  \},\n  "noble_language")',
        "".join(lines) + r"\1",
        text,
        count=1,
    )
    BARKS.write_text(text, encoding="utf-8")
    return len(lines)


if __name__ == "__main__":
    print(complete_faction_language(), "factions ajoutées")
