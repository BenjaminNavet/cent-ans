"""Lot FK1: data/rules/map_scenes.json and the `map_scene` / `presentation` event keys."""

import copy
import json
import re
from pathlib import Path

from jsonschema import Draft202012Validator
from referencing import Registry, Resource

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict:
    return json.loads(path.read_text(encoding="utf-8"))


def _event_validator() -> Draft202012Validator:
    """Event schema validator resolving `common.schema.json` locally."""
    registry = Registry()
    for path in (DATA / "schemas").glob("*.schema.json"):
        schema = _load(path)
        resource = Resource.from_contents(schema)
        registry = registry.with_resource(schema["$id"], resource).with_resource(
            path.name, resource
        )
    schema = _load(DATA / "schemas" / "event.schema.json")
    return Draft202012Validator(schema, registry=registry)


def _rules_validator() -> Draft202012Validator:
    schema = _load(DATA / "schemas" / "map_scenes_rules.schema.json")
    Draft202012Validator.check_schema(schema)
    return Draft202012Validator(schema)


def test_map_scene_rules_match_schema() -> None:
    """The rules file exists, matches its schema and times every scene kind."""
    rules = _load(DATA / "rules" / "map_scenes.json")
    kinds = _load(DATA / "schemas" / "map_scenes_rules.schema.json")["$defs"][
        "map_scene"
    ]["enum"]
    assert set(rules["durations"]) == set(kinds)


def test_scene_kinds_agree_between_schemas() -> None:
    """The closed list of scenes is the same for events and for the rules."""
    events = _load(DATA / "schemas" / "event.schema.json")["properties"]["map_scene"]
    rules = _load(DATA / "schemas" / "map_scenes_rules.schema.json")["$defs"][
        "map_scene"
    ]
    assert events["enum"] == rules["enum"]


def test_unknown_scene_duration_is_refused() -> None:
    """A duration for an unknown scene kind is refused."""
    rules = copy.deepcopy(_load(DATA / "rules" / "map_scenes.json"))
    rules["durations"]["dragon"] = 2
    assert list(_rules_validator().iter_errors(rules))


def test_unknown_map_scene_or_presentation_is_refused() -> None:
    """An event with an unknown `map_scene` or `presentation` is refused."""
    validator = _event_validator()
    event = _load(DATA / "events" / "evt_crue.json")
    assert event["map_scene"] == "flood"
    assert not list(validator.iter_errors(event))
    bad_scene = dict(event, map_scene="dragon")
    assert list(validator.iter_errors(bad_scene))
    bad_presentation = dict(event, presentation="popup")
    assert list(validator.iter_errors(bad_presentation))
    good_presentation = dict(event, presentation="dialog")
    assert not list(validator.iter_errors(good_presentation))


def test_tagged_events() -> None:
    """A few existing events carry their scene (spec § 2.1.1)."""
    expected = {
        "evt_black_death": "plague",
        "evt_crue": "flood",
        "evt_disette": "famine",
        "evt_foire_prospere": "fair",
        "evt_sacre_de_reims": "celebration",
    }
    for event_id, scene in expected.items():
        assert _load(DATA / "events" / f"{event_id}.json")["map_scene"] == scene


def test_renderer_keys_are_in_the_schema() -> None:
    """Every tuning key the renderer reads (life_folk) is declared by the schema (FK4).

    `data/rules/map_scenes.json` is the single source of the living map tuning: a key
    read by `game/scripts/map/life_folk/*.gd` but absent from the schema would be a
    silent fallback to a value hard-coded in the renderer.
    """
    folk_dir = DATA.parent / "game" / "scripts" / "map" / "life_folk"
    pattern = re.compile(r'(?:settings\.get|_setting)\("([a-z_]+)"')
    read = set()
    for path in folk_dir.glob("*.gd"):
        read |= set(pattern.findall(path.read_text(encoding="utf-8")))
    assert read, "no tuning key found in the renderer"
    declared = set(
        _load(DATA / "schemas" / "map_scenes_rules.schema.json")["properties"]
    )
    assert read <= declared, sorted(read - declared)
    rules = _load(DATA / "rules" / "map_scenes.json")
    assert read <= set(rules), sorted(read - set(rules))
