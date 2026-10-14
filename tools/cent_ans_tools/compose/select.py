"""Lot C0a : sélection des lieux et fusion des provinces de la carte composée (ADR 0355).

Prototype : ne lit que ``data/`` et n'écrit que ``data/map/compose/selection.json`` et
``id_map.json``. Étapes :

1. emprise Europe + Méditerranée : retrait des provinces trop à l'est (capitale à
   ``lon > max_lon``) ou d'une région-tag de la Horde, du Caucase, de la Mésopotamie ;
2. fusion en ``province_target`` provinces : seulement entre voisines (adjacence de
   ``provinces.geojson``) de même ``owner``, en absorbant d'abord la moins importante ;
3. sélection de ``settlement_target`` lieux : une cité par province, des lieux protégés,
   puis des lieux secondaires par score avec un espacement minimal réel.
"""

from __future__ import annotations

import json
import math
from collections import Counter, defaultdict
from dataclasses import dataclass, field
from pathlib import Path

from cent_ans_tools import settlement_cap

REPO_DIR = settlement_cap.REPO_DIR
DATA_DIR = settlement_cap.DATA_DIR
OUT_DIR = DATA_DIR / "map" / "compose"

DEFAULT_MAX_LON = 41.5
DROPPED_REGIONS = ("horde", "caucase", "mesopotamie")
MAX_GROUP = 5
KEEP_PROVINCE_IDS = (
    "prov_ile_de_france",
    "prov_guyenne",
    "prov_normandie",
    "prov_kent",
    "prov_jerusalem",
)
# Détenteurs de l'Ouest jouable : leur quota de provinces est moins comprimé (lot C3b).
WEST_OWNERS = [
    "fac_france",
    "fac_england",
    "fac_burgundy",
    "fac_brittany",
    "fac_flanders",
    "fac_navarre",
    "fac_scotland",
    "fac_aragon",
    "fac_castile",
    "fac_portugal",
    "fac_armagnac",
]
WEST_QUOTA_FACTOR = 0.75
# Province fusionnée -> cité imposée (cible de croisade, ADR 0355 / lot C3b).
FORCED_CITY = {"prov_jerusalem": "set_jerusalem"}
TEST_SETTLEMENTS = (
    "set_paris",
    "set_saint_denis",
    "set_meaux",
    "set_southampton",
    "set_bordeaux",
    "set_calais",
    "set_boulogne",
    "set_rouen",
    "set_dover",
)
PROTECTED_SOURCES = (
    "data/naval/*.json",
    "data/economy/trade.json",
    "data/rules/crusade.json",
    "data/rules/capture.json",
)
SECONDARY_KINDS = ("town", "castle", "abbey", "village")
# Quota de lieux secondaires selon le centile d'importance de la province.
QUOTA_STEPS = ((0.40, 1), (0.75, 2), (0.92, 3), (1.01, 4))
ZONES = {
    "France": (
        "france_nord france_centre france_ouest france_est aquitaine languedoc "
        "provence_alpes"
    ),
    "Îles Britanniques": (
        "angleterre_sud angleterre_nord angleterre_centre angleterre_est galles ecosse irlande"
    ),
    "Ibérie": "aragon iberie_nord iberie_centre iberie_sud portugal",
    "Italie": "italie_nord italie_centre italie_sud",
    "Empire": "empire_rhin empire_sud empire_est empire_nord pays_bas",
    "Scandinavie": "scandinavie norvege finlande",
    "Est": (
        "pologne lituanie rus_centre rus_nord rus_ouest rus_sud hongrie prusse pomeranie "
        "livonie mazovie valachie transylvanie crimee silesie"
    ),
    "Balkans/Anatolie/Levant": "balkans grece egee croatie anatolie levant chypre",
    "Maghreb": "maghreb egypte",
}


@dataclass
class Params:
    """Paramètres de la sélection."""

    province_target: int = 213
    settlement_target: int = 400
    min_spacing_km: float = 45.0
    max_lon: float = DEFAULT_MAX_LON
    dropped_regions: tuple[str, ...] = DROPPED_REGIONS
    # "owner" : fusion entre provinces de même owner (spec) ; "realm" : même suzerain final ;
    # "tiered" (défaut, C3b) : même owner d'abord, puis même suzerain, puis n'importe quel voisin.
    merge_scope: str = "tiered"

    def as_dict(self) -> dict:
        """Représentation JSON."""
        return {
            "province_target": self.province_target,
            "settlement_target": self.settlement_target,
            "min_spacing_km": self.min_spacing_km,
            "max_lon": self.max_lon,
            "dropped_regions": list(self.dropped_regions),
            "max_group": MAX_GROUP,
            "merge_scope": self.merge_scope,
        }


@dataclass
class Data:
    """Entrées chargées de ``data/``."""

    provinces: dict[str, dict]
    adjacency: dict[str, set[str]]
    settlements: dict[str, list[dict]]
    factions: dict[str, dict]
    title_of: dict[str, str]
    liege_of: dict[str, str]
    rules: dict
    markers: dict
    protected: set[str]
    references: set[str]


@dataclass
class Result:
    """Sortie de :func:`select`."""

    selection: dict
    id_map: dict
    notes: dict = field(default_factory=dict)


def _load_dir(pattern: str, data_dir: Path) -> list[dict]:
    return [
        json.loads(p.read_text(encoding="utf-8"))
        for p in sorted(data_dir.glob(pattern))
    ]


def load(data_dir: Path = DATA_DIR, repo_dir: Path = REPO_DIR) -> Data:
    """Lit provinces, adjacence, colonies, factions, titres et règles de score."""
    provinces = {p["id"]: p for p in _load_dir("provinces/*.json", data_dir)}
    # Adjacence d'ORIGINE (443 provinces réelles) : depuis C4b, data/map/provinces.geojson est celui de
    # la carte composée ; l'ancien est dans data/map/real/ (compile.ensure_real_sources).
    geojson_path = data_dir / "map" / "provinces.geojson"
    real_geojson = data_dir / "map" / "real" / "provinces.geojson"
    if real_geojson.is_file():
        geojson_path = real_geojson
    geojson = json.loads(geojson_path.read_text(encoding="utf-8"))
    adjacency: dict[str, set[str]] = defaultdict(set)
    for feature in geojson["features"]:
        props = feature["properties"]
        for other in props["neighbors"]:
            adjacency[props["id"]].add(other)
            adjacency[other].add(props["id"])
    settlements = settlement_cap.load_all(data_dir)
    title_of, liege_of = {}, {}
    for title in _load_dir("titles/*.json", data_dir):
        for pid in title.get("de_jure_provinces", []):
            title_of[pid] = title["id"]
            liege_of[pid] = title.get("de_jure_liege") or title["id"]
    rules = settlement_cap.load_rules(data_dir / "map" / "settlement_cap_rules.json")
    markers = json.loads(
        (data_dir / "map" / "settlement_markers.json").read_text(encoding="utf-8")
    )
    ids = {e["id"] for entries in settlements.values() for e in entries}
    return Data(
        provinces=provinces,
        adjacency=adjacency,
        settlements=settlements,
        factions={f["id"]: f for f in _load_dir("factions/*.json", data_dir)},
        title_of=title_of,
        liege_of=liege_of,
        rules=rules,
        markers=markers,
        protected=settlement_cap.referenced_ids(list(PROTECTED_SOURCES), ids, repo_dir),
        references=settlement_cap.referenced_ids(
            rules["reference_sources"], ids, repo_dir
        ),
    )


def haversine_km(a: tuple[float, float], b: tuple[float, float]) -> float:
    """Distance orthodromique entre deux (lon, lat) en km."""
    lon1, lat1, lon2, lat2 = map(math.radians, (*a, *b))
    h = (
        math.sin((lat2 - lat1) / 2) ** 2
        + math.cos(lat1) * math.cos(lat2) * math.sin((lon2 - lon1) / 2) ** 2
    )
    return 12742.0 * math.asin(math.sqrt(h))


def marker_rank(entry: dict, markers: dict) -> int:
    """Rang de carte (settlement_markers.json) d'une colonie."""
    best = markers.get("default_rank", 1)
    for rule in markers["rank_rules"]:
        if "ids" in rule:
            hit = entry["id"] in rule["ids"]
        else:
            hit = (
                entry["kind"] == rule.get("kind", entry["kind"])
                and entry.get("fortification_level", 0)
                >= rule.get("min_fortification", 0)
                and entry["weight"] >= rule.get("min_weight", 0)
            )
        if hit:
            best = max(best, rule["rank"])
    return best


def kept_provinces(data: Data, params: Params) -> tuple[list[str], list[str]]:
    """(provinces gardées, provinces retirées) selon l'emprise."""
    kept, dropped = [], []
    for pid, province in sorted(data.provinces.items()):
        out = (
            province["capital_city"]["lon"] > params.max_lon
            or province["region"] in params.dropped_regions
        )
        (dropped if out else kept).append(pid)
    return kept, dropped


def faction_capitals(data: Data) -> set[str]:
    """Identifiants des provinces capitales de faction."""
    return {f["capital"] for f in data.factions.values() if f.get("capital")}


def scope_key(data: Data, faction: str, params: Params) -> str:
    """Clé de compatibilité de fusion : le détenteur, ou son suzerain final (scope realm)."""
    if params.merge_scope != "realm":
        return faction
    seen: set[str] = set()
    while faction not in seen and data.factions.get(faction, {}).get("suzerain"):
        seen.add(faction)
        faction = data.factions[faction]["suzerain"]
    return faction


def merge_provinces(
    data: Data,
    kept: list[str],
    scores: dict[str, float],
    params: Params,
) -> dict[str, list[str]]:
    """Groupes de fusion ``{id fusionné: [provinces d'origine]}``.

    On absorbe d'abord le groupe le moins important dans son voisin de même ``owner``,
    de préférence de même région puis de même titre de jure (sinon le plus important).
    Un groupe dont l'id est protégé n'est jamais absorbé.
    """
    protected_ids = faction_capitals(data) | set(KEEP_PROVINCE_IDS)
    groups = {pid: [pid] for pid in kept}
    importance = {pid: scores.get(pid, 0.0) for pid in kept}
    group_of = {pid: pid for pid in kept}

    def neighbor_groups(lead: str) -> set[str]:
        found = set()
        for member in groups[lead]:
            for other in data.adjacency.get(member, ()):
                if other in group_of and group_of[other] != lead:
                    found.add(group_of[other])
        return found

    def affinity(small: str, big: str) -> tuple:
        a, b = data.provinces[small], data.provinces[big]
        return (
            a["region"] == b["region"],
            data.liege_of.get(small) == data.liege_of.get(big),
            data.title_of.get(small) == data.title_of.get(big),
        )

    def tier_key(province_id: str, tier: int) -> str:
        if tier == 0:
            return data.provinces[province_id]["owner"]
        if tier == 1:
            return scope_key(data, data.provinces[province_id]["owner"], params)
        return ""

    if params.merge_scope == "tiered":
        tiers = (0, 1, 2)
    elif params.merge_scope == "owner":
        tiers = (0,)
    else:
        tiers = (1,)

    before_count = Counter(data.provinces[pid]["owner"] for pid in kept)

    def merge_pass(tier: int, ratio: float | None = None) -> None:
        """Fusionne au niveau ``tier`` ; avec ``ratio``, un détenteur garde au moins n/ratio groupes."""
        held = Counter(data.provinces[g]["owner"] for g in groups)
        while len(groups) > params.province_target:
            best = None
            for small in sorted(groups, key=lambda g: (importance[g], g)):
                if small in protected_ids:
                    continue
                small_owner = data.provinces[small]["owner"]
                if ratio is not None and held[small_owner] <= max(
                    1,
                    round(
                        before_count[small_owner]
                        / (
                            ratio * WEST_QUOTA_FACTOR
                            if small_owner in WEST_OWNERS
                            else ratio
                        )
                    ),
                ):
                    continue
                options = [
                    big
                    for big in neighbor_groups(small)
                    if tier_key(owner_of[big], tier) == tier_key(owner_of[small], tier)
                    and len(groups[big]) + len(groups[small]) <= MAX_GROUP
                ]
                if options:
                    big = max(
                        options,
                        key=lambda g: (
                            affinity(small, g),
                            -len(groups[g]),
                            importance[g],
                            g,
                        ),
                    )
                    best = (small, big)
                    break
            if best is None:
                return
            small, big = best
            held[data.provinces[small]["owner"]] -= 1
            groups[big] += groups.pop(small)
            importance[big] += importance.pop(small)
            for member in groups[big]:
                group_of[member] = big

    # Le détenteur d'un groupe reste celui de sa tête pendant la fusion.
    owner_of = {pid: pid for pid in kept}
    if params.merge_scope == "tiered":
        # Niveau « même détenteur », réparti au prorata du poids d'origine de chaque détenteur.
        base_ratio = len(kept) / params.province_target
        for step in range(0, 41):
            for tier in tiers:
                merge_pass(tier, base_ratio * (1 + 0.03 * step))
            if len(groups) <= params.province_target:
                break
    for tier in tiers:
        merge_pass(tier)
    return finalize_groups(data, groups, scores, set(kept))


def population_of(province: dict) -> int:
    """Population totale d'une province."""
    return sum(row["count"] for row in province["population"]["classes"].values())


def finalize_groups(
    data: Data,
    groups: dict[str, list[str]],
    scores: dict[str, float],
    kept: set[str],
) -> dict[str, list[str]]:
    """Choisit la tête (id gardé) de chaque groupe : la plus importante du détenteur majoritaire.

    Détenteur = faction qui possède le plus de provinces d'origine du groupe (égalité :
    population, puis la tête actuelle). Un détenteur dont la capitale est hors emprise ne
    l'emporte que s'il est seul. Une tête protégée (capitale de faction, id à garder)
    reste en place et garde son propre détenteur.
    """
    protected_ids = faction_capitals(data) | set(KEEP_PROVINCE_IDS)
    result: dict[str, list[str]] = {}
    for lead, members in groups.items():
        if lead in protected_ids or len(members) == 1:
            result[lead] = sorted(members)
            continue
        count: Counter = Counter()
        population: Counter = Counter()
        for member in members:
            owner = data.provinces[member]["owner"]
            count[owner] += 1
            population[owner] += population_of(data.provinces[member])
        lead_owner = data.provinces[lead]["owner"]
        owner = max(
            count,
            key=lambda o: (
                data.factions.get(o, {}).get("capital", next(iter(kept))) in kept,
                count[o],
                population[o],
                o == lead_owner,
                o,
            ),
        )
        new_lead = max(
            (m for m in members if data.provinces[m]["owner"] == owner),
            key=lambda m: (scores.get(m, 0.0), m == lead, m),
        )
        result[new_lead] = sorted(members)
    return dict(sorted(result.items()))


def spacing_ok(
    point: tuple[float, float], taken: list[tuple[float, float]], minimum: float
) -> bool:
    """Vrai si ``point`` est à au moins ``minimum`` km de tous les points pris."""
    return all(haversine_km(point, other) >= minimum for other in taken)


def quotas(importance: dict[str, float]) -> dict[str, int]:
    """Quota de lieux secondaires (1 à 4) par centile d'importance."""
    ranked = sorted(importance, key=lambda g: (importance[g], g))
    result = {}
    for index, lead in enumerate(ranked):
        percentile = (index + 0.5) / len(ranked)
        result[lead] = next(q for limit, q in QUOTA_STEPS if percentile < limit)
    return result


def pick_city(
    members: list[str],
    data: Data,
    entries_by_prov: dict[str, list[dict]],
    score,
    owner: str | None = None,
) -> dict | None:
    """Cité de la province fusionnée : d'un membre du détenteur, capitale de faction, de province, score."""
    capitals = {
        f["capital"]: f.get("capital_city")
        for f in data.factions.values()
        if f.get("capital")
    }
    best, best_key = None, None
    for pid in members:
        province_capital = data.provinces[pid]["capital_city"]["name"]["display"]
        for entry in entries_by_prov.get(pid, []):
            if entry["kind"] != "city":
                continue
            name = entry["name"]["display"]
            key = (
                owner is None or data.provinces[pid]["owner"] == owner,
                pid in capitals and capitals[pid] == name,
                name == province_capital,
                score(entry),
                entry["id"],
            )
            if best_key is None or key > best_key:
                best, best_key = entry, key
    return best


def select(
    params: Params | None = None,
    data: Data | None = None,
) -> Result:
    """Calcule la sélection complète (déterministe)."""
    params = params or Params()
    data = data or load()
    kept, dropped = kept_provinces(data, params)

    def score(entry: dict) -> float:
        return settlement_cap.score(entry, data.rules, data.references)

    # Importance d'une province = somme des scores de ses lieux.
    scores = {pid: sum(score(e) for e in data.settlements.get(pid, [])) for pid in kept}
    groups = merge_provinces(data, kept, scores, params)
    prov_to_group = {m: lead for lead, members in groups.items() for m in members}
    importance = {
        lead: sum(scores[m] for m in members) for lead, members in groups.items()
    }
    quota = quotas(importance)

    entries_by_group: dict[str, list[dict]] = {
        lead: [e for m in members for e in data.settlements.get(m, [])]
        for lead, members in groups.items()
    }
    entries_by_prov = {pid: data.settlements.get(pid, []) for pid in kept}
    pos = {e["id"]: tuple(e["lonlat"]) for es in entries_by_group.values() for e in es}
    must_keep = set(data.protected) | set(TEST_SETTLEMENTS)
    must_keep |= {
        e["id"]
        for es in entries_by_group.values()
        for e in es
        if marker_rank(e, data.markers) >= 3
    }

    chosen: dict[str, dict] = {}  # id -> entry
    chosen_in: dict[str, list[str]] = defaultdict(list)
    taken: list[tuple[float, float]] = []

    def take(lead: str, entry: dict) -> None:
        chosen[entry["id"]] = entry
        chosen_in[lead].append(entry["id"])
        taken.append(pos[entry["id"]])

    # 1. Une cité par province fusionnée.
    city_of: dict[str, str] = {}
    for lead, members in groups.items():
        city = pick_city(
            members, data, entries_by_prov, score, data.provinces[lead]["owner"]
        )
        forced = FORCED_CITY.get(lead)
        if forced:
            city = next((e for e in entries_by_group[lead] if e["id"] == forced), city)
        if city:
            take(lead, city)
            city_of[lead] = city["id"]

    def secondaries(lead: str) -> list[dict]:
        return sorted(
            (
                e
                for e in entries_by_group[lead]
                if e["id"] not in chosen
                and (e["kind"] in SECONDARY_KINDS or e["kind"] == "city")
            ),
            key=lambda e: (-score(e), e["id"]),
        )

    def secondary_count(lead: str) -> int:
        return len(chosen_in[lead]) - (1 if lead in city_of else 0)

    # 2. Lieux protégés par priorité (tests, rang 3-4, données), plafond 4 secondaires,
    # espacement ignoré entre eux.
    tests = set(TEST_SETTLEMENTS)

    def tier(entry: dict) -> int:
        if entry["id"] in tests:
            return 0
        return 1 if marker_rank(entry, data.markers) >= 3 else 2

    protected_entries = sorted(
        (
            (lead, e)
            for lead in groups
            for e in entries_by_group[lead]
            if e["id"] in must_keep and e["id"] not in chosen
        ),
        key=lambda pair: (tier(pair[1]), -score(pair[1]), pair[1]["id"]),
    )
    for lead, entry in protected_entries:
        if entry["id"] not in chosen and secondary_count(lead) < 4:
            take(lead, entry)

    # 3. Au moins un secondaire par province, puis remplissage global.
    def fill(limit_for: dict[str, int], honor_target: bool) -> None:
        candidates = sorted(
            (
                (lead, e)
                for lead in groups
                for e in entries_by_group[lead]
                if e["id"] not in chosen
            ),
            key=lambda pair: (-score(pair[1]), pair[1]["id"]),
        )
        for lead, entry in candidates:
            if honor_target and len(chosen) >= params.settlement_target:
                return
            if secondary_count(lead) >= limit_for[lead]:
                continue
            if not spacing_ok(pos[entry["id"]], taken, params.min_spacing_km):
                continue
            take(lead, entry)

    # Passe « un par province » (villages en dernier recours).
    for lead in sorted(groups, key=lambda g: (-importance[g], g)):
        if len(chosen) >= params.settlement_target:
            break
        if secondary_count(lead) >= 1:
            continue
        options = [e for e in secondaries(lead) if e["kind"] != "village"] or [
            e for e in secondaries(lead) if e["kind"] == "village"
        ]
        for entry in options:
            if spacing_ok(pos[entry["id"]], taken, params.min_spacing_km):
                take(lead, entry)
                break
    for extra in range(0, 4):
        if len(chosen) >= params.settlement_target:
            break
        fill({lead: min(4, q + extra) for lead, q in quota.items()}, True)

    # Sortie.
    settlements_out = []
    for lead in groups:
        for sid in chosen_in[lead]:
            entry = chosen[sid]
            kind = entry["kind"]
            record = {
                "id": sid,
                "province": lead,
                "kind": "town"
                if (kind == "city" and city_of.get(lead) != sid)
                else kind,
                "lon": entry["lonlat"][0],
                "lat": entry["lonlat"][1],
                "score": round(score(entry), 2),
                "protected": sid in must_keep,
            }
            if record["kind"] != kind:
                record["former_kind"] = kind
            settlements_out.append(record)

    provinces_out = []
    for lead, members in groups.items():
        province = data.provinces[lead]
        provinces_out.append(
            {
                "id": lead,
                "merged_from": members,
                "owner": province["owner"],
                "region": province["region"],
                "capital_settlement": city_of.get(lead),
                "settlements": chosen_in[lead],
            }
        )
        owners = sorted({data.provinces[m]["owner"] for m in members})
        if len(owners) > 1:
            provinces_out[-1]["owners"] = owners

    owners_before = Counter(p["owner"] for p in data.provinces.values())
    owners_after = Counter(data.provinces[pid]["owner"] for pid in kept)
    dropped_factions = sorted(o for o in owners_before if o not in owners_after)
    lost_capital = sorted(
        f["id"]
        for f in data.factions.values()
        if f.get("capital") in dropped and f["id"] in owners_after
    )

    id_map = {
        "provinces": {pid: prov_to_group.get(pid) for pid in sorted(data.provinces)},
        "settlements": {
            e["id"]: (e["id"] if e["id"] in chosen else None)
            for pid in sorted(data.settlements)
            for e in data.settlements[pid]
        },
    }
    selection = {
        "params": params.as_dict(),
        "provinces": provinces_out,
        "settlements": settlements_out,
        "dropped_provinces": dropped,
        "dropped_factions": dropped_factions,
    }
    notes = {
        "factions_lost_capital": lost_capital,
        "protected_missing": sorted(
            sid for sid in must_keep if sid not in chosen and sid in pos
        ),
        "protected_unknown": sorted(
            sid for sid in set(TEST_SETTLEMENTS) if sid not in pos
        ),
    }
    return Result(selection=selection, id_map=id_map, notes=notes)


def zone_of(region: str) -> str:
    """Grande zone d'un tag de région."""
    for zone, tags in ZONES.items():
        if region in tags.split():
            return zone
    return "Autre"


def nearest_gaps(selection: dict) -> list[float]:
    """Distance (km) de chaque lieu retenu à son plus proche voisin retenu."""
    pts = [(s["lon"], s["lat"]) for s in selection["settlements"]]
    return [
        min(haversine_km(p, q) for j, q in enumerate(pts) if j != i)
        for i, p in enumerate(pts)
    ]


def write(result: Result, out_dir: Path = OUT_DIR, suffix: str = "") -> None:
    """Écrit selection<suffix>.json et id_map<suffix>.json."""
    out_dir.mkdir(parents=True, exist_ok=True)
    for name, value in (
        (f"selection{suffix}.json", result.selection),
        (f"id_map{suffix}.json", result.id_map),
    ):
        (out_dir / name).write_text(
            json.dumps(value, ensure_ascii=False, indent=1) + "\n", encoding="utf-8"
        )
