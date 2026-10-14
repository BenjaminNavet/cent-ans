"""Lot C6 : recalage des données de la carte composée (échelle, fusion, textes).

Trois passes rejouables (idempotentes, elles repartent des données d'avant composition
lues dans git, ``BASE_REF``, ou d'une table explicite) :

* :func:`base_supply` : ``base_supply`` d'une province fusionnée = moyenne pondérée par la
  population de ses membres (grandeur locale, voir ``docs/wip/cc-c3-donnees.md`` § C6) ;
* :func:`rivers` : abscisses ``km`` de ``data/movement/rivers.json`` mises à l'échelle de jeu
  (rapport entre la distance de jeu et la distance réelle entre les extrémités du fleuve) ;
* :func:`texts` : réécriture des textes de jeu qui nomment une province ou un lieu disparu
  (table explicite) ; ``--audit`` compte les mentions restantes par dossier.
"""

from __future__ import annotations

import json
import math
import re
import subprocess
from collections import Counter
from pathlib import Path
from typing import Any

REPO_DIR = Path(__file__).resolve().parents[3]
DATA_DIR = REPO_DIR / "data"
#: Dernier commit d'avant composition (443 provinces, 1656 lieux).
BASE_REF = "8e251a874"
EARTH_RADIUS_KM = 6371.0088


def _git_json(path: str) -> Any:
    out = subprocess.run(
        ["git", "show", f"{BASE_REF}:{path}"],
        cwd=REPO_DIR,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return json.loads(out)


def _dump(path: Path, data: Any) -> None:
    path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )


def _population(province: dict) -> int:
    return sum(row["count"] for row in province["population"]["classes"].values())


def base_supply(dry_run: bool = False) -> dict[str, tuple[int, int]]:
    """Recalcule ``base_supply`` des provinces fusionnées ; retourne ``{id: (avant, après)}``."""
    id_map = json.loads((DATA_DIR / "map/compose/id_map.json").read_text())["provinces"]
    members: dict[str, list[dict]] = {}
    for old_id, new_id in id_map.items():
        if new_id is None:
            continue
        members.setdefault(new_id, []).append(
            _git_json(f"data/provinces/{old_id}.json")
        )
    changes: dict[str, tuple[int, int]] = {}
    for new_id, group in sorted(members.items()):
        path = DATA_DIR / "provinces" / f"{new_id}.json"
        if not path.exists():
            continue
        current = json.loads(path.read_text())
        weights = [max(_population(m), 1) for m in group]
        values = [m.get("base_supply", 0) for m in group]
        new = (
            round(
                sum(w * v for w, v in zip(weights, values, strict=True))
                / sum(weights)
                / 50
            )
            * 50
        )
        before = current.get("base_supply", 0)
        if new != before:
            changes[new_id] = (before, new)
            if not dry_run:
                current["base_supply"] = new
                _dump(path, current)
    return changes


def _haversine_km(a: list[float], b: list[float]) -> float:
    lat1, lat2 = math.radians(a[1]), math.radians(b[1])
    dlat, dlon = lat2 - lat1, math.radians(b[0] - a[0])
    h = (
        math.sin(dlat / 2) ** 2
        + math.cos(lat1) * math.cos(lat2) * math.sin(dlon / 2) ** 2
    )
    return 2 * EARTH_RADIUS_KM * math.asin(min(1.0, math.sqrt(h)))


def rivers(dry_run: bool = False) -> list[tuple[str, float]]:
    """Met les ``km`` des fleuves navigables à l'échelle de jeu ; retourne ``[(fleuve, rapport)]``."""
    mpp = json.loads((DATA_DIR / "map/map.json").read_text())["meters_per_px"]
    px = json.loads((DATA_DIR / "map/settlements_px.json").read_text())
    lonlat = {}
    for path in (DATA_DIR / "settlements").glob("prov_*.json"):
        for place in json.loads(path.read_text()):
            lonlat[place["id"]] = place["lonlat"]
    base = {
        r["id"]: {p["settlement"]: p["km"] for p in r["places"]}
        for r in _git_json("data/movement/rivers.json")["rivers"]
    }
    path = DATA_DIR / "movement/rivers.json"
    document = json.loads(path.read_text())
    ratios = []
    for river in document["rivers"]:
        places = river["places"]
        first, last = places[0]["settlement"], places[-1]["settlement"]
        game = math.dist(px[first], px[last]) * mpp / 1000.0
        real = _haversine_km(lonlat[first], lonlat[last])
        ratio = game / real if real > 1.0 else 1.0
        ratios.append((river["id"], round(ratio, 3)))
        origin = base[river["id"]][first]
        for place in places:
            original = base[river["id"]][place["settlement"]]
            place["km"] = round(origin + (original - origin) * ratio, 1)
    if not dry_run:
        _dump(path, document)
    return ratios


#: (fichier relatif à ``data/``, ancien texte, nouveau texte) : textes de jeu qui désignent
#: une cible de jeu disparue. L'histoire (maisons, personnes, récits) n'est pas réécrite.
TEXT_FIXES: list[tuple[str, str, str]] = [
    (
        "missions.json",
        "Tenir Reims et la Champagne",
        "Tenir l’Île-de-France",
    ),
    (
        "missions.json",
        "Gardez la Champagne, où se font les rois : le sacre de Charles VII à Reims (17 juillet 1429) fonde sa légitimité.",
        "Gardez l’Île-de-France (qui absorbe la Champagne), où se font les rois : le sacre de Charles VII à Reims (17 juillet 1429) fonde sa légitimité.",
    ),
    (
        "missions.json",
        "Obtenez la Picardie et les villes de la Somme, cédées",
        "Obtenez le Ponthieu (Amiens, Abbeville) et les villes de la Somme, cédées",
    ),
    (
        "events/evt_sacre_de_reims.json",
        "Exige de tenir la Champagne, où se trouve Reims.",
        "Exige de tenir l’Île-de-France (avec la Champagne), où se trouve Reims.",
    ),
    (
        "tutorial/steps.json",
        "Tenez Paris et Reims, reprenez la Guyenne",
        "Tenez Paris et l’Île-de-France, reprenez la Guyenne",
    ),
    (
        "tutorial/steps.json",
        "Faites-vous sacrer à Reims, ce qu'Henri VI",
        "Faites-vous sacrer (à Reims, en Île-de-France), ce qu'Henri VI",
    ),
]

#: Champs ``province`` de missions à recaler (la province fusionnée désignait le mauvais pays).
MISSION_PROVINCES = {"bg_picardie": "prov_ponthieu"}


def texts(dry_run: bool = False) -> list[str]:
    """Applique :data:`TEXT_FIXES` et :data:`MISSION_PROVINCES` ; retourne les fichiers changés."""
    changed = []
    by_file: dict[str, list[tuple[str, str]]] = {}
    for rel, old, new in TEXT_FIXES:
        by_file.setdefault(rel, []).append((old, new))
    for rel, fixes in by_file.items():
        path = DATA_DIR / rel
        text = path.read_text(encoding="utf-8")
        original = text
        for old, new in fixes:
            text = text.replace(old, new)
        if rel == "missions.json":
            for mission_id, province in MISSION_PROVINCES.items():
                text = re.sub(
                    r'("id": "' + mission_id + r'".*?"province": ")prov_\w+(")',
                    r"\g<1>" + province + r"\g<2>",
                    text,
                    count=1,
                    flags=re.S,
                )
        if text != original:
            changed.append(rel)
            if not dry_run:
                path.write_text(text, encoding="utf-8")
    return changed


def audit() -> Counter[str]:
    """Mentions restantes de noms de provinces disparues, par dossier de ``data/``."""
    id_map = json.loads((DATA_DIR / "map/compose/id_map.json").read_text())["provinces"]
    surviving = {p.stem for p in (DATA_DIR / "provinces").glob("prov_*.json")}
    current_names = {
        json.loads(p.read_text())["name"]["display"]
        for p in (DATA_DIR / "provinces").glob("prov_*.json")
    }
    removed = {}
    for old_id in id_map:
        if old_id in surviving:
            continue
        name = _git_json(f"data/provinces/{old_id}.json")["name"]["display"]
        if name not in current_names:
            removed[name] = re.compile(
                r"(?<![\wÀ-ÿ])" + re.escape(name) + r"(?![\wÀ-ÿ])"
            )
    counts: Counter[str] = Counter()
    for path in DATA_DIR.rglob("*.json"):
        rel = path.relative_to(DATA_DIR)
        if rel.parts[0] in {"map", "schemas", "provinces", "settlements"}:
            continue
        text = path.read_text(encoding="utf-8")
        for pattern in removed.values():
            counts[rel.parts[0]] += len(pattern.findall(text))
    return counts
