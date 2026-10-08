"""Feudal title registry (lot FE, ADR 0098): schemas, invariants."""

import json
from collections import Counter
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

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
    errors = [
        e.message for e in _validator("feudal_rules.schema.json").iter_errors(rules)
    ]
    assert not errors, errors


def test_lieges_exist_and_rank_strictly_higher() -> None:
    """A title's liege exists and is of strictly higher rank (no cycle, 3 levels)."""
    titles = _load("titles")
    for title in titles.values():
        liege = title.get("de_jure_liege")
        if liege is None:
            continue
        assert liege in titles, f"{title['id']}: unknown liege {liege}"
        assert RANK_ORDER[titles[liege]["rank"]] > RANK_ORDER[title["rank"]], title[
            "id"
        ]


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
