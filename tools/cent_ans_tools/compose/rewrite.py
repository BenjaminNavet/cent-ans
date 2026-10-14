"""Lot C3 : réécriture des données de jeu pour la carte composée (ADR 0355).

Lit ``data/map/compose/id_map.json`` et ``selection.json`` (C0a) et réécrit en place
``data/provinces``, ``data/settlements``, factions, personnages, titres, événements et
toutes les références (naval, commerce, fleuves, règles...). Idempotent : un
identifiant absent de la table est inchangé, et les provinces absorbées n'existent plus
au second passage.

Règles d'agrégation (détail dans ``docs/wip/cc-c3-donnees.md``) :

* province gardée = celle que donne ``id_map`` ; propriétaire, culture, religion, terrain,
  climat, titre : ceux de la province gardée ;
* population : effectifs additionnés par classe, taux (révolte, santé, richesse,
  ravitaillement) moyennés au prorata des effectifs ; ``base_supply`` additionné ;
  ressources, fleuves, ports, mers : union ; ``fortification_level`` et ``buildings`` de la
  province gardée (ils s'appliquent à sa cité) ;
* lieu gardé : copié ; kind et province selon la sélection ; venu d'une province de
  propriétaire différent, il devient une enclave (``owner``) ; l'ancienne cité gardée comme
  lieu secondaire reçoit les bâtiments et la fortification de son ancienne province ;
* lieux non retenus : retirés (les hameaux de ``data/map/`` sont régénérés en C4).
"""

from __future__ import annotations

import heapq
import json
import math
import re
from collections import Counter
from collections.abc import Callable
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

REPO_DIR = Path(__file__).resolve().parents[3]
DATA_DIR = REPO_DIR / "data"

RATE_FIELDS = ("unrest", "health", "wealth", "goods_satisfaction")
# Clés dont la valeur peut disparaître seule sans emporter l'objet qui la porte.
OPTIONAL_KEYS = {
    "suzerain",
    "starting_location",
    "entity",
    "heir",
    "ruler",
    "primary_title",
    "head_faction",
}
# Dossiers et fichiers de ``data/`` qui ne portent aucune référence à réécrire ici.
SKIP_TOP = {"map", "schemas", "provinces", "settlements"}
SKIP_FILES = {"codex_bundle.json"}
# Fichiers qui nomment des régions de province (clés ou listes).
REGION_FILES = (
    "art/town_maquettes.json",
    "art/tx_building_pack.json",
    "rules/mercenaries.json",
    "portraits/archetypes.json",
)


class _Drop:
    """Marque « cette référence a disparu »."""

    def __repr__(self) -> str:
        return "DROP"


DROP = _Drop()


# ---------------------------------------------------------------------------
# Fichiers JSON (format conservé)


def detect_indent(text: str) -> int | None:
    """Indentation du fichier d'origine (``None`` : compact)."""
    lines = text.split("\n")
    if len(lines) <= 2:
        return None
    match = re.match(r" +", lines[1])
    return len(match.group(0)) if match else None


@dataclass
class Doc:
    """Un fichier JSON lu, avec le format nécessaire pour le réécrire à l'identique."""

    path: Path
    data: Any
    indent: int | None

    def dump(self) -> str:
        if self.indent is None:
            text = json.dumps(self.data, ensure_ascii=False, separators=(",", ":"))
        else:
            text = json.dumps(self.data, ensure_ascii=False, indent=self.indent)
        return text + "\n"


def read_doc(path: Path) -> Doc:
    text = path.read_text(encoding="utf-8")
    return Doc(path, json.loads(text), detect_indent(text))


def clone(value: Any) -> Any:
    return json.loads(json.dumps(value))


# ---------------------------------------------------------------------------
# Rapport


@dataclass
class Report:
    before: Counter = field(default_factory=Counter)
    after: Counter = field(default_factory=Counter)
    removed_factions: list[str] = field(default_factory=list)
    removed_characters: list[str] = field(default_factory=list)
    removed_titles: list[str] = field(default_factory=list)
    kept_empty_titles: list[str] = field(default_factory=list)
    removed_events: list[str] = field(default_factory=list)
    notes: list[str] = field(default_factory=list)
    written: int = 0
    deleted: int = 0

    def summary_lines(self) -> list[str]:
        lines = []
        for key in sorted(set(self.before) | set(self.after)):
            lines.append(f"{key} : {self.before[key]} -> {self.after[key]}")
        lines.append(f"factions retirées : {', '.join(self.removed_factions)}")
        lines.append(f"personnages retirés : {', '.join(self.removed_characters)}")
        lines.append(f"titres retirés ({len(self.removed_titles)}) : {', '.join(self.removed_titles)}")
        lines.append(f"titres vidés gardés : {', '.join(self.kept_empty_titles)}")
        lines.append(f"événements retirés : {', '.join(self.removed_events)}")
        lines.extend(self.notes)
        lines.append(f"fichiers écrits : {self.written}, supprimés : {self.deleted}")
        return lines


def haversine_km(a: list[float], b: list[float]) -> float:
    (lon1, lat1), (lon2, lat2) = a, b
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = p2 - p1, math.radians(lon2 - lon1)
    h = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 12742.0 * math.asin(math.sqrt(h))


# ---------------------------------------------------------------------------


class Rewriter:
    """Calcule le plan, réécrit les entités puis toutes les références."""

    def __init__(self, data_dir: Path = DATA_DIR, dry_run: bool = False) -> None:
        self.data_dir = data_dir
        self.dry_run = dry_run
        self.report = Report()
        compose = data_dir / "map" / "compose"
        id_map = json.loads((compose / "id_map.json").read_text(encoding="utf-8"))
        self.selection = json.loads((compose / "selection.json").read_text(encoding="utf-8"))
        self.prov_map: dict[str, str | None] = id_map["provinces"]
        self.set_map: dict[str, str | None] = id_map["settlements"]
        self.ref: dict[str, Any] = {}  # ancien -> nouveau, ou DROP
        self.targets: set[str] = {v for v in (*self.prov_map.values(), *self.set_map.values()) if v}
        self.pending_writes: dict[Path, Doc] = {}
        self.pending_deletes: set[Path] = set()
        self.new_files: dict[str, list[dict]] = {}
        self.new_provinces: dict[str, dict] = {}
        self.all_settlements: dict[str, dict] = {}  # anciens lieux, tous
        self.kept_settlement_province: dict[str, str] = {}
        self.region_ref: dict[str, Any] = {}

    # -- E/S ---------------------------------------------------------------

    def _stage(self, doc: Doc) -> None:
        self.pending_writes[doc.path] = doc

    def _delete(self, path: Path) -> None:
        self.pending_deletes.add(path)

    def _load_dir(self, folder: str) -> dict[str, Doc]:
        docs = {}
        for path in sorted((self.data_dir / folder).glob("*.json")):
            if path.name in ("rules.json", "vocations.json"):
                continue
            docs[path.stem] = read_doc(path)
        return docs

    # -- Références --------------------------------------------------------

    def walk(self, obj: Any, report: list[str], path: str = "") -> Any:
        """Réécrit récursivement les identifiants d'un objet JSON.

        Une référence disparue retire l'élément de liste qui la porte, ou la clé si elle est
        facultative ; sinon elle remonte jusqu'à l'élément englobant (``DROP``).
        """
        if isinstance(obj, str):
            if obj in self.ref:
                new = self.ref[obj]
                if new is DROP:
                    report.append(f"{path}={obj}")
                return new
            return obj
        if isinstance(obj, list):
            out: list[Any] = []
            for item in obj:
                new = self.walk(item, report, path + "[]")
                if new is DROP:
                    continue
                if isinstance(item, str) and (item in self.ref or item in self.targets) and new in out:
                    continue
                out.append(new)
            return out
        if isinstance(obj, dict):
            out_dict: dict[str, Any] = {}
            for key, value in obj.items():
                new_key = key
                if key in self.ref:
                    new_key = self.ref[key]
                    if new_key is DROP:
                        continue
                    if new_key in obj or new_key in out_dict:
                        continue  # l'entrée de la province gardée l'emporte
                new_value = self.walk(value, report, f"{path}.{key}")
                if new_value is DROP:
                    if key in OPTIONAL_KEYS:
                        continue
                    return DROP
                out_dict[new_key] = new_value
            return out_dict
        return obj

    # -- Plan --------------------------------------------------------------

    def run(self) -> Report:
        self._rewrite_core()
        self._rewrite_references()
        self._rewrite_regions()
        self._rewrite_graph()
        self.report.after["événements"] -= len(self.report.removed_events)
        self._flush()
        return self.report

    def _rewrite_core(self) -> None:
        data = self.data_dir
        rep = self.report
        sel = self.selection
        provinces = self._load_dir("provinces")
        settlement_files = self._load_dir("settlements")
        factions = self._load_dir("factions")
        characters = self._load_dir("characters")
        titles = self._load_dir("titles")
        rep.before.update(
            {
                "provinces": len(provinces),
                "lieux": sum(len(d.data) for d in settlement_files.values()),
                "factions": len(factions),
                "personnages": len(characters),
                "titres": len(titles),
                "événements": len(list((data / "events").glob("*.json"))),
            }
        )
        self.all_settlements = {s["id"]: s for d in settlement_files.values() for s in d.data}
        kept_sets = {s["id"]: s for s in sel["settlements"]}

        # 1. Provinces et lieux fusionnés ----------------------------------------
        for entry in sel["provinces"]:
            pid = entry["id"]
            if pid not in provinces:
                continue
            members = [m for m in entry["merged_from"] if m in provinces]
            if any(m not in provinces for m in entry["merged_from"]) and len(members) > 1:
                rep.notes.append(f"{pid} : membres partiellement absents")
            base = provinces[pid].data
            absorbed = [provinces[m].data for m in members if m != pid]
            self.new_provinces[pid] = self._merge_province(base, absorbed, entry)
            self.new_files[pid] = self._build_settlements(entry, base, provinces, kept_sets)
            for s in self.new_files[pid]:
                self.kept_settlement_province[s["id"]] = pid

        # 1b. Régions disparues -> région de la province qui a absorbé la plupart des leurs.
        region_votes: dict[str, Counter] = {}
        for old_id, doc in provinces.items():
            new_id = self.prov_map.get(old_id)
            if new_id is None or new_id not in self.new_provinces:
                region_votes.setdefault(doc.data["region"], Counter())
                continue
            region_votes.setdefault(doc.data["region"], Counter())[
                self.new_provinces[new_id]["region"]
            ] += 1
        surviving = {p["region"] for p in self.new_provinces.values()}
        for region, votes in region_votes.items():
            if region not in surviving:
                self.region_ref[region] = votes.most_common(1)[0][0] if votes else DROP
        rep.notes.append(f"régions disparues : {self.region_ref}")

        # 2. Table de références -----------------------------------------------------
        for table in (self.prov_map, self.set_map):
            for old, new in table.items():
                if new != old:
                    self.ref[old] = DROP if new is None else new

        # 3. Factions sans territoire ---------------------------------------------
        new_owners = {p["owner"] for p in self.new_provinces.values()}
        enclave_owners = {s["owner"] for lst in self.new_files.values() for s in lst if s.get("owner")}
        owners_before = {d.data["owner"] for d in provinces.values()}
        removed_factions = set(sel["dropped_factions"])
        for fid in factions:
            if fid in owners_before and fid not in new_owners and fid not in enclave_owners:
                removed_factions.add(fid)
        removed_factions &= set(factions)
        rep.removed_factions = sorted(removed_factions)
        for fid in removed_factions:
            self.ref[fid] = DROP

        # 4. Personnages ----------------------------------------------------------------
        removed_chars = {
            cid for cid, d in characters.items() if d.data["faction"] in removed_factions
        }
        rep.removed_characters = sorted(removed_chars)
        for cid in removed_chars:
            self.ref[cid] = DROP

        # 5. Titres -----------------------------------------------------------------------
        survivors = set(factions) - removed_factions
        landed = new_owners | enclave_owners
        removed_titles = self._plan_titles(titles, factions, survivors, landed)
        rep.removed_titles = sorted(removed_titles)
        reparent = self._reparent_map(titles, removed_titles)
        for tid in removed_titles:
            self.ref[tid] = DROP

        # 6. Écriture des entités ------------------------------------------------------
        for pid, doc in provinces.items():
            if pid not in self.new_provinces:
                self._delete(doc.path)
        for pid, prov in self.new_provinces.items():
            self._stage(Doc(provinces[pid].path, prov, provinces[pid].indent))
        for stem, doc in settlement_files.items():
            if stem not in self.new_files:
                self._delete(doc.path)
        for pid, lst in self.new_files.items():
            old = settlement_files.get(pid)
            path = old.path if old else data / "settlements" / f"{pid}.json"
            self._stage(Doc(path, lst, old.indent if old else 2))
        for fid in removed_factions:
            self._delete(factions[fid].path)
        for cid in removed_chars:
            self._delete(characters[cid].path)
        for tid in removed_titles:
            self._delete(titles[tid].path)

        # 7. Entités restantes -----------------------------------------------------------
        for fid, doc in factions.items():
            if fid not in removed_factions:
                self._rewrite_faction(doc)
        for cid, doc in characters.items():
            if cid not in removed_chars:
                self._rewrite_generic(doc)
        for tid, doc in titles.items():
            if tid not in removed_titles:
                self._rewrite_title(doc, reparent)

        rep.after.update(
            {
                "provinces": len(self.new_provinces),
                "lieux": sum(len(v) for v in self.new_files.values()),
                "factions": len(factions) - len(removed_factions),
                "personnages": len(characters) - len(removed_chars),
                "titres": len(titles) - len(removed_titles),
            }
        )
        rep.after["événements"] = rep.before["événements"]

    # -- Provinces -----------------------------------------------------------

    @staticmethod
    def _union(*lists: list[Any]) -> list[Any]:
        out: list[Any] = []
        for lst in lists:
            for item in lst or []:
                if item not in out:
                    out.append(item)
        return out

    def _merge_province(self, base: dict, absorbed: list[dict], entry: dict) -> dict:
        merged = clone(base)
        members = [base, *absorbed]
        neighbors = []
        for member in members:
            for n in member.get("neighbors", []):
                new = self.prov_map.get(n, n)
                if new is not None and new != base["id"] and new not in neighbors:
                    neighbors.append(new)
        merged["neighbors"] = neighbors
        for key in ("rivers", "ports", "sea_zones", "resources"):
            if any(key in m for m in members):
                merged[key] = self._union(*[m.get(key, []) for m in members])
        merged["coastal"] = any(m.get("coastal") for m in members)
        merged["base_supply"] = sum(m.get("base_supply", 0) for m in members)
        merged["geo"]["voronoi_weight"] = max(
            m.get("geo", {}).get("voronoi_weight", 0) for m in members
        )
        capital = self.all_settlements[entry["capital_settlement"]]
        merged["capital_city"] = {
            "name": capital["name"],
            "lat": capital["lonlat"][1],
            "lon": capital["lonlat"][0],
        }
        merged["geo"]["capital_lonlat"] = list(capital["lonlat"])
        classes = {}
        for cls, base_row in base["population"]["classes"].items():
            rows = [m["population"]["classes"][cls] for m in members]
            total = sum(r["count"] for r in rows)
            row = dict(base_row)
            row["count"] = total
            for rate in RATE_FIELDS:
                if rate in base_row:
                    weighted = sum(r["count"] * r.get(rate, 0) for r in rows)
                    row[rate] = round(weighted / total) if total else base_row[rate]
            classes[cls] = row
        merged["population"]["classes"] = classes
        if absorbed:
            merged["population"]["uncertain"] = True
            names = ", ".join(m["name"]["display"] for m in absorbed)
            merged["description"] = (
                base["description"].rstrip() + f" Province composée : regroupe aussi {names}."
            )
            merged["sources"] = self._union(*[m.get("sources", []) for m in members])
        return merged

    def _build_settlements(
        self,
        entry: dict,
        base: dict,
        provinces: dict[str, Doc],
        kept_sets: dict[str, dict],
    ) -> list[dict]:
        pid = entry["id"]
        ids = [entry["capital_settlement"]] + [
            s for s in entry["settlements"] if s != entry["capital_settlement"]
        ]
        out = []
        for sid in ids:
            settlement = clone(self.all_settlements[sid])
            old_prov = provinces[settlement["province"]].data
            was_city = settlement["kind"] == "city" and settlement["province"] != pid
            if settlement["kind"] == "city" and settlement["province"] == pid:
                was_city = False  # la cité de la province gardée : déjà dans ``buildings``
            settlement["province"] = pid
            settlement["kind"] = kept_sets[sid]["kind"]
            if was_city:
                settlement["buildings"] = self._union(
                    old_prov.get("buildings", []), settlement["buildings"]
                )
                settlement["fortification_level"] = max(
                    settlement["fortification_level"], old_prov.get("fortification_level", 0)
                )
            if settlement.get("owner") is None and old_prov["owner"] != base["owner"]:
                settlement["owner"] = old_prov["owner"]
            if settlement.get("owner") == base["owner"]:
                settlement["owner"] = None
            out.append(settlement)
        return out

    # -- Factions, personnages, titres ------------------------------------

    def _rewrite_faction(self, doc: Doc) -> None:
        notes: list[str] = []
        new = self.walk(doc.data, notes)
        if new is DROP:
            self.report.notes.append(f"{doc.path.name} : walk l'a supprimée ({notes}), inchangée")
            return
        owned = {pid for pid, p in self.new_provinces.items() if p["owner"] == new["id"]}
        if "claims" in new:
            new["claims"] = [c for c in new["claims"] if c.get("province") not in owned]
            if not new["claims"]:
                del new["claims"]
        victory = new.get("victory")
        if victory is not None and not victory.get("objectives"):
            del new["victory"]
        capital = self.new_provinces.get(new.get("capital"))
        if capital is not None and new.get("capital_city") is not None:
            names = {s["name"]["display"] for s in self.new_files[new["capital"]]}
            if new["capital_city"] not in names:
                new["capital_city"] = capital["capital_city"]["name"]["display"]
        doc.data = new
        self._stage(doc)

    def _rewrite_generic(self, doc: Doc) -> None:
        notes: list[str] = []
        new = self.walk(doc.data, notes)
        if new is DROP:
            self.report.notes.append(f"{doc.path.name} : entièrement dépendant d'une référence perdue ({notes})")
            return
        doc.data = new
        self._stage(doc)

    def _plan_titles(
        self, titles: dict[str, Doc], factions: dict[str, Doc], survivors: set[str], landed: set[str]
    ) -> set[str]:
        primary = {
            f.data["primary_title"]
            for fid, f in factions.items()
            if fid in survivors and fid in landed and f.data.get("primary_title")
        }
        removed: set[str] = set()
        for tid, doc in titles.items():
            data = doc.data
            if data["holder_1337"]["faction"] not in survivors:
                removed.add(tid)
            elif not [p for p in data["de_jure_provinces"] if self._survives(p)]:
                if tid in primary:
                    self.report.kept_empty_titles.append(tid)
                else:
                    removed.add(tid)
        return removed

    def _survives(self, pid: str) -> bool:
        return self.prov_map.get(pid, pid) == pid

    @staticmethod
    def _reparent_map(titles: dict[str, Doc], removed: set[str]) -> dict[str, str | None]:
        liege = {tid: d.data.get("de_jure_liege") for tid, d in titles.items()}
        out: dict[str, str | None] = {}
        for tid in titles:
            if tid in removed:
                continue
            up = liege[tid]
            while up in removed:
                up = liege.get(up)
            out[tid] = up
        return out

    def _rewrite_title(self, doc: Doc, reparent: dict[str, str | None]) -> None:
        data = doc.data
        data["de_jure_provinces"] = [p for p in data["de_jure_provinces"] if self._survives(p)]
        if reparent[data["id"]] is None:
            data.pop("de_jure_liege", None)
        else:
            data["de_jure_liege"] = reparent[data["id"]]
        notes: list[str] = []
        new = self.walk(data, notes)
        if new is DROP:
            self.report.notes.append(f"{data['id']} : walk l'a supprimé ({notes}), inchangé")
            return
        objectives = [
            o for o in new.get("objectives", []) if not self._empty_condition(o)
        ]
        if "objectives" in new:
            if objectives:
                new["objectives"] = objectives
            else:
                del new["objectives"]
        doc.data = new
        self._stage(doc)

    @staticmethod
    def _empty_condition(objective: dict) -> bool:
        condition = objective.get("condition", {})
        return "provinces" in condition and not condition["provinces"]

    # -- Références dans le reste de data/ ----------------------------------

    def _rewrite_references(self) -> None:
        pattern = re.compile("|".join(re.escape(k) for k in sorted(self.ref, key=len, reverse=True)))
        for path in sorted(self.data_dir.rglob("*.json")):
            rel = path.relative_to(self.data_dir)
            if rel.parts[0] in SKIP_TOP or rel.name in SKIP_FILES:
                continue
            if rel.parts[0] in ("factions", "characters", "titles"):
                continue
            if path in self.pending_writes or path in self.pending_deletes:
                continue
            text = path.read_text(encoding="utf-8")
            if not pattern.search(text) and rel.as_posix() not in SPECIAL:
                continue
            doc = Doc(path, json.loads(text), detect_indent(text))
            if rel.parts[0] == "events":
                self._rewrite_event(doc)
                continue
            handler = SPECIAL.get(rel.as_posix())
            if handler is not None:
                handler(self, doc)
            else:
                self._rewrite_generic(doc)

    def _rewrite_regions(self) -> None:
        """Régions disparues dans les fichiers qui les nomment (listes, clés, dossiers)."""
        if not self.region_ref:
            return
        files = [self.data_dir / f for f in REGION_FILES]
        files += sorted((self.data_dir / "encounters").glob("*.json"))
        saved = self.ref, self.targets
        self.ref = self.region_ref
        self.targets = {v for v in self.region_ref.values() if v is not DROP}
        try:
            for path in files:
                if not path.exists() or path in self.pending_deletes:
                    continue
                doc = self.pending_writes.get(path) or read_doc(path)
                notes: list[str] = []
                new = self.walk(doc.data, notes)
                if new is not DROP:
                    doc.data = new
                    self._stage(doc)
        finally:
            self.ref, self.targets = saved

    def _rewrite_graph(self) -> None:
        """Contracte ``map/settlement_graph.json`` sur les lieux retenus (provisoire jusqu'à C4).

        Les lieux retirés sont traversés : deux lieux retenus sont reliés par le chemin le plus
        court qui ne passe que par des lieux retirés (coût cumulé, route si tout le chemin en
        est une, mer si un tronçon en est un). Une arête est élaguée si un tiers la double à
        moins de 5 %. Aucune géométrie : la chaîne de C4 régénère ce fichier.
        """
        path = self.data_dir / "map" / "settlement_graph.json"
        if not path.exists():
            return
        doc = read_doc(path)
        kept = {s["id"] for lst in self.new_files.values() for s in lst}
        adjacency: dict[str, list[tuple[str, float, bool, bool]]] = {}
        for edge in doc.data["edges"]:
            for a, b in ((edge["from"], edge["to"]), (edge["to"], edge["from"])):
                adjacency.setdefault(a, []).append((b, edge["cost"], edge["road"], edge["sea"]))
        links: dict[tuple[str, str], tuple[float, bool, bool]] = {}
        for start in sorted(kept):
            best = {start: (0.0, True, False)}
            queue = [(0.0, start)]
            while queue:
                dist, node = heapq.heappop(queue)
                if dist > best[node][0] or (node != start and node in kept):
                    continue
                for other, cost, road, sea in adjacency.get(node, []):
                    cand = dist + cost
                    if other not in best or cand < best[other][0]:
                        best[other] = (cand, best[node][1] and road, best[node][2] or sea)
                        heapq.heappush(queue, (cand, other))
            for other, (dist, road, sea) in best.items():
                if other != start and other in kept:
                    key = tuple(sorted((start, other)))
                    if key not in links or links[key][0] > dist:
                        links[key] = (round(dist, 2), road, sea)  # type: ignore[assignment]
        near: dict[str, dict[str, float]] = {}
        for (a, b), (cost, _, _) in links.items():
            near.setdefault(a, {})[b] = cost
            near.setdefault(b, {})[a] = cost
        edges = []
        for (a, b), (cost, road, sea) in sorted(links.items()):
            if any(
                x in near[b] and near[a][x] + near[b][x] <= cost * 1.05
                for x in near[a]
                if x != b
            ):
                continue
            edges.append({"from": a, "to": b, "cost": cost, "road": road, "sea": sea})
        self.report.notes.append(
            f"graphe des lieux contracté : {len(doc.data['edges'])} -> {len(edges)} arêtes"
        )
        doc.data = {"edges": edges}
        self._stage(doc)

    def _rewrite_event(self, doc: Doc) -> None:
        notes: list[str] = []
        new = self.walk(doc.data, notes)
        if notes or new is DROP:
            self.report.removed_events.append(doc.data["id"])
            self._delete(doc.path)
            return
        doc.data = new
        self._stage(doc)

    # -- Cas particuliers ------------------------------------------------------

    def province_city(self, old_settlement_id: str) -> str | None:
        """Cité de la province (fusionnée) qui contenait ce lieu."""
        old_prov = self.all_settlements[old_settlement_id]["province"]
        new_prov = self.prov_map.get(old_prov, old_prov)
        if new_prov is None:
            return None
        return next(
            (s["id"] for s in self.new_files.get(new_prov, []) if s["kind"] == "city"), None
        )

    def nearest_kept(self, old_settlement_id: str, *, port: bool) -> str | None:
        """Lieu retenu le plus proche dans la même province fusionnée (port si demandé)."""
        old = self.all_settlements[old_settlement_id]
        new_prov = self.prov_map.get(old["province"], old["province"])
        candidates = [
            s
            for s in self.new_files.get(new_prov, [])
            if (s.get("port") or not port)
        ]
        if not candidates:
            return None
        return min(candidates, key=lambda s: haversine_km(old["lonlat"], s["lonlat"]))["id"]

    def _flush(self) -> None:
        for path, doc in sorted(self.pending_writes.items()):
            if path in self.pending_deletes:
                continue
            text = doc.dump()
            if path.exists() and path.read_text(encoding="utf-8") == text:
                continue
            self.report.written += 1
            if not self.dry_run:
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text(text, encoding="utf-8")
        for path in sorted(self.pending_deletes):
            self.report.deleted += 1
            if not self.dry_run and path.exists():
                path.unlink()
        if not self.dry_run:
            from cent_ans_tools import codex

            codex.write_bundle(self.data_dir)


# ---------------------------------------------------------------------------
# Fichiers à traitement particulier


def _trade(rw: Rewriter, doc: Doc) -> None:
    """Un comptoir sans lieu retenu passe à la cité de sa province ; ses routes suivent."""
    data = doc.data
    used = {h["settlement"] for h in data["hubs"] if h["settlement"] not in rw.ref}
    for hub in data["hubs"]:
        sid = hub["settlement"]
        if rw.ref.get(sid) is DROP:
            replacement = rw.province_city(sid)
            if replacement and replacement not in used:
                hub["settlement"] = replacement
                used.add(replacement)
    rw._rewrite_generic(doc)
    hubs = {h["id"] for h in doc.data["hubs"]}
    doc.data["routes"] = [
        r for r in doc.data["routes"] if r["from_hub"] in hubs and r["to_hub"] in hubs
    ]


def _sea_lanes(rw: Rewriter, doc: Doc) -> None:
    """Un port retiré est remplacé par le port retenu le plus proche de sa province."""
    lanes = []
    seen: set[frozenset[str]] = set()
    for lane in doc.data["lanes"]:
        for end in ("from", "to"):
            if rw.ref.get(lane[end]) is DROP:
                replacement = rw.nearest_kept(lane[end], port=True)
                if replacement is None:
                    break
                lane[end] = replacement
        else:
            if lane["from"] == lane["to"]:
                continue
            key = frozenset((lane["from"], lane["to"], lane["sea"]))
            if key in seen:
                continue
            seen.add(key)
            lanes.append(lane)
            continue
        rw.report.notes.append(f"route maritime {lane['id']} retirée (port perdu)")
    doc.data["lanes"] = lanes
    rw._rewrite_generic(doc)


def _rivers(rw: Rewriter, doc: Doc) -> None:
    rw._rewrite_generic(doc)
    kept = []
    for river in doc.data["rivers"]:
        if len(river["places"]) >= 2:
            kept.append(river)
        else:
            rw.report.notes.append(f"fleuve {river['id']} retiré (moins de deux places)")
    doc.data["rivers"] = kept


def _building_regions(rw: Rewriter, doc: Doc) -> None:
    """Les régions sans province survivante n'ont plus de matériaux de bâtiment."""
    rw._rewrite_generic(doc)
    used = {p["region"] for p in rw.new_provinces.values()}
    doc.data["regions"] = {k: v for k, v in doc.data["regions"].items() if k in used}


SPECIAL: dict[str, Callable[[Rewriter, Doc], None]] = {
    "economy/trade.json": _trade,
    "naval/sea_lanes.json": _sea_lanes,
    "movement/rivers.json": _rivers,
    "art/building_regions.json": _building_regions,
}


def run(dry_run: bool = False, data_dir: Path = DATA_DIR) -> Report:
    """Point d'entrée de ``cent-ans compose rewrite``."""
    return Rewriter(data_dir, dry_run).run()
