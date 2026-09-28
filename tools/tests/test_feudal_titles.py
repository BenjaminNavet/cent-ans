"""Feudal title registry (lot FE, ADR 0098): schemas, invariants, migration."""

import json
from collections import Counter
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

from cent_ans_tools.feudal_migrate import plan_migration

DATA = Path(__file__).resolve().parents[2] / "data"
RANK_ORDER = {"county": 0, "duchy": 1, "kingdom": 2}


def _validator(schema_name: str) -> Draft202012Validator:
    """Validator resolving the other schemas of `data/schemas/` locally."""
    schemas = {
        path.name: json.loads(path.read_text(encoding="utf-8"))
        for path in (DATA / "schemas").glob("*.schema.json")
    }
    registry = Registry()
    for name, schema in schemas.items():
        resource = Resource.from_contents(schema)
        registry = registry.with_resource(schema["$id"], resource).with_resource(
            name, resource
        )
    return Draft202012Validator(schemas[schema_name], registry=registry)


def _load(folder: str) -> dict[str, dict]:
    return {
        path.stem: json.loads(path.read_text(encoding="utf-8"))
        for path in sorted((DATA / folder).glob("*.json"))
    }


def test_titles_factions_provinces_and_rules_match_their_schemas() -> None:
    """Titles, factions, provinces and the feudal rules are schema-valid."""
    for folder, schema in [
        ("titles", "title.schema.json"),
        ("factions", "faction.schema.json"),
        ("provinces", "province.schema.json"),
    ]:
        validator = _validator(schema)
        for stem, entity in _load(folder).items():
            errors = [error.message for error in validator.iter_errors(entity)]
            assert not errors, f"{folder}/{stem}: {errors}"
            assert stem == entity["id"]
    rules = json.loads((DATA / "rules" / "feudal.json").read_text(encoding="utf-8"))
    errors = [e.message for e in _validator("feudal_rules.schema.json").iter_errors(rules)]
    assert not errors, errors


def test_lieges_exist_and_rank_strictly_higher() -> None:
    """A title's liege exists and is of strictly higher rank (no cycle, 3 levels)."""
    titles = _load("titles")
    for title in titles.values():
        liege = title.get("de_jure_liege")
        if liege is None:
            continue
        assert liege in titles, f"{title['id']}: unknown liege {liege}"
        assert RANK_ORDER[titles[liege]["rank"]] > RANK_ORDER[title["rank"]], title["id"]


def test_every_province_is_in_exactly_one_title() -> None:
    """No orphan province, no province in two titles."""
    provinces = _load("provinces")
    counts = Counter(
        province
        for title in _load("titles").values()
        for province in title.get("de_jure_provinces", [])
    )
    assert set(counts) == set(provinces), set(provinces) ^ set(counts)
    assert all(count == 1 for count in counts.values()), [
        p for p, c in counts.items() if c > 1
    ]


def test_owning_factions_hold_their_primary_title() -> None:
    """Every faction owning a province has a primary title it holds in 1337."""
    titles = _load("titles")
    owners = {province["owner"] for province in _load("provinces").values()}
    for faction_id, faction in _load("factions").items():
        primary = faction.get("primary_title")
        if primary is None:
            assert faction_id not in owners, f"{faction_id} has no primary_title"
            continue
        assert titles[primary]["holder_1337"]["faction"] == faction_id


def test_no_province_keeps_overlord_or_holder() -> None:
    """The migration removed the deduced fields."""
    for province in _load("provinces").values():
        assert "overlord" not in province and "holder" not in province


def test_migration_splits_a_fief_and_an_appanage() -> None:
    """Guyenne-like fief and Normandy-like appanage get their own titles."""

    def faction(fid: str, government: str, ruler: str, suzerain: str | None = None) -> dict:
        entity = {
            "id": fid,
            "name": {"display": fid},
            "government": government,
            "ruler": ruler,
            "heraldry": {"blazon": "x", "primary_color": "#000000"},
        }
        if suzerain:
            entity["suzerain"] = suzerain
        return entity

    factions = {
        "fac_fr": faction("fac_fr", "kingdom", "chr_king_fr"),
        "fac_en": faction("fac_en", "kingdom", "chr_king_en"),
        "fac_bz": faction("fac_bz", "duchy", "chr_duke", "fac_fr"),
    }
    provinces = {
        "prov_paris": {"id": "prov_paris", "name": {"display": "Paris"}, "owner": "fac_fr"},
        "prov_rouen": {
            "id": "prov_rouen",
            "name": {"display": "Rouen"},
            "owner": "fac_fr",
            "holder": "chr_prince",
        },
        "prov_london": {"id": "prov_london", "name": {"display": "Londres"}, "owner": "fac_en"},
        "prov_bordeaux": {
            "id": "prov_bordeaux",
            "name": {"display": "Bordeaux"},
            "owner": "fac_en",
            "overlord": "fac_fr",
        },
        "prov_rennes": {
            "id": "prov_rennes",
            "name": {"display": "Rennes"},
            "owner": "fac_bz",
            "overlord": "fac_fr",
        },
    }
    migration = plan_migration(factions, provinces)
    titles = migration.titles
    assert titles["tit_fr"]["rank"] == "kingdom"
    assert titles["tit_fr"]["de_jure_provinces"] == ["prov_paris"]
    assert titles["tit_bz"]["de_jure_liege"] == "tit_fr"
    assert titles["tit_bz"]["de_jure_provinces"] == ["prov_rennes"]
    assert titles["tit_bordeaux"]["de_jure_liege"] == "tit_fr"
    assert titles["tit_bordeaux"]["holder_1337"]["faction"] == "fac_en"
    assert titles["tit_rouen"]["de_jure_liege"] == "tit_fr"
    assert titles["tit_rouen"]["holder_1337"]["faction"] == "fac_fr"
    assert migration.primary_titles == {"fac_fr": "tit_fr", "fac_en": "tit_en", "fac_bz": "tit_bz"}
