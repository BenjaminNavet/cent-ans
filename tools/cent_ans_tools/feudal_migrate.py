"""One-shot migration to feudal titles (lot FE0, ADR 0098).

Reads the pre-FE data (``owner``, ``overlord`` and ``holder`` of provinces,
``suzerain`` of factions) and writes the title registry ``data/titles/``:

- one primary title per faction owning provinces: a sovereign title
  (rank ``kingdom``) for a faction without suzerain, a ``duchy`` or
  ``county`` (by government) under its suzerain's primary title otherwise.
  A faction without suzerain whose provinces all sit under the same foreign
  overlord (the imperial princes) is a *de jure* vassal of that overlord;
- one secondary title per province held under another overlord than the
  owner's primary chain (Guyenne: England under France), or by a character
  other than the faction's ruler (Normandy: John, Duke of Normandy);
- every other province in its owner's primary domain.

It then adds ``primary_title`` to the factions and removes ``overlord`` and
``holder`` from the provinces. Provinces are left unchanged otherwise.
"""

from __future__ import annotations

import json
from collections import Counter
from dataclasses import dataclass, field
from pathlib import Path

RANKS = ["county", "duchy", "kingdom"]
COUNTY_GOVERNMENTS = {"county", "lordship", "republic"}


@dataclass
class Migration:
    """Result of :func:`plan_migration`: titles to write, faction primary titles."""

    titles: dict[str, dict] = field(default_factory=dict)
    primary_titles: dict[str, str] = field(default_factory=dict)
    notes: list[str] = field(default_factory=list)


def _suffix(entity_id: str) -> str:
    return entity_id.split("_", 1)[1]


def _effective_overlord(province: dict) -> str | None:
    overlord = province.get("overlord")
    return None if overlord == province["owner"] else overlord


def _rank_below(rank: str) -> str | None:
    index = RANKS.index(rank)
    return RANKS[index - 1] if index > 0 else None


def plan_migration(factions: dict[str, dict], provinces: dict[str, dict]) -> Migration:
    """Builds the title registry from pre-FE factions and provinces."""
    migration = Migration()
    owned: dict[str, list[dict]] = {}
    for province in provinces.values():
        owned.setdefault(province["owner"], []).append(province)

    # De jure liege faction of each owning faction.
    liege_faction: dict[str, str | None] = {}
    for faction_id in owned:
        faction = factions[faction_id]
        liege = faction.get("suzerain")
        if liege is None:
            overlords = Counter(_effective_overlord(p) for p in owned[faction_id])
            common, count = overlords.most_common(1)[0]
            if common is not None and count == len(owned[faction_id]):
                liege = common
                migration.notes.append(f"{faction_id}: de jure vassal of {common}")
        liege_faction[faction_id] = liege

    # Primary titles: sovereigns first so that vassals can point to them.
    def primary_id(faction_id: str) -> str:
        return f"tit_{_suffix(faction_id)}"

    order = sorted(owned, key=lambda f: (liege_faction[f] is not None, f))
    for faction_id in order:
        faction = factions[faction_id]
        liege = liege_faction[faction_id]
        if liege is None:
            rank = "kingdom"
        else:
            rank = "county" if faction["government"] in COUNTY_GOVERNMENTS else "duchy"
        title = {
            "id": primary_id(faction_id),
            "rank": rank,
            "name": faction["name"],
            "heraldry": faction["heraldry"],
        }
        if liege is not None:
            title["de_jure_liege"] = primary_id(liege)
        title["de_jure_provinces"] = []
        title["holder_1337"] = {"faction": faction_id}
        if faction.get("sources"):
            title["holder_1337"]["sources"] = faction["sources"]
        migration.titles[title["id"]] = title
        migration.primary_titles[faction_id] = title["id"]

    # Provinces: primary domain or a secondary title.
    for province_id in sorted(provinces):
        province = provinces[province_id]
        owner = province["owner"]
        faction = factions[owner]
        primary = migration.titles[primary_id(owner)]
        overlord = _effective_overlord(province)
        holder = province.get("holder")
        foreign = overlord is not None and overlord != liege_faction[owner]
        appanage = holder is not None and holder != faction.get("ruler")
        if not foreign and not appanage:
            primary["de_jure_provinces"].append(province_id)
            continue
        liege_title = migration.titles[primary_id(overlord)] if overlord and foreign else primary
        rank = _rank_below(liege_title["rank"])
        if rank is None:
            migration.notes.append(f"{province_id}: no rank below {liege_title['id']}, kept in {primary['id']}")
            primary["de_jure_provinces"].append(province_id)
            continue
        title_id = f"tit_{_suffix(province_id)}"
        if title_id in migration.titles:
            title_id = f"{title_id}_{rank}"
        if title_id in migration.titles:
            raise ValueError(f"title id collision: {title_id}")
        title = {
            "id": title_id,
            "rank": rank,
            "name": province["name"],
            "de_jure_liege": liege_title["id"],
            "de_jure_provinces": [province_id],
            "holder_1337": {"faction": owner},
        }
        if province.get("sources"):
            title["holder_1337"]["sources"] = province["sources"]
        if appanage:
            title["description"] = f"Tenu en 1337 par {holder} (apanage ou fief)."
        migration.titles[title_id] = title
    return migration


def _read_dir(folder: Path) -> dict[str, dict]:
    return {
        path.stem: json.loads(path.read_text(encoding="utf-8"))
        for path in sorted(folder.glob("*.json"))
    }


def _write(path: Path, value: dict) -> None:
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def _with_primary_title(faction: dict, title_id: str) -> dict:
    """Inserts ``primary_title`` right after ``suzerain`` (or ``succession_law``)."""
    anchor = "suzerain" if "suzerain" in faction else "succession_law"
    result: dict = {}
    for key, value in faction.items():
        if key == "primary_title":
            continue
        result[key] = value
        if key == anchor:
            result["primary_title"] = title_id
    return result


def migrate(data_dir: Path) -> Migration:
    """Runs the migration in place on ``data_dir`` (refuses to run twice)."""
    titles_dir = data_dir / "titles"
    if titles_dir.exists() and any(titles_dir.glob("*.json")):
        raise FileExistsError(f"{titles_dir} already exists: migration already done")
    factions = _read_dir(data_dir / "factions")
    provinces = _read_dir(data_dir / "provinces")
    migration = plan_migration(factions, provinces)
    titles_dir.mkdir(exist_ok=True)
    for title_id, title in migration.titles.items():
        _write(titles_dir / f"{title_id}.json", title)
    for faction_id, title_id in migration.primary_titles.items():
        _write(
            data_dir / "factions" / f"{faction_id}.json",
            _with_primary_title(factions[faction_id], title_id),
        )
    for province_id, province in provinces.items():
        if "overlord" in province or "holder" in province:
            cleaned = {k: v for k, v in province.items() if k not in ("overlord", "holder")}
            _write(data_dir / "provinces" / f"{province_id}.json", cleaned)
    return migration


if __name__ == "__main__":
    result = migrate(Path(__file__).resolve().parents[2] / "data")
    print(f"{len(result.titles)} titles, {len(result.primary_titles)} primary titles")
    for note in result.notes:
        print(note)
