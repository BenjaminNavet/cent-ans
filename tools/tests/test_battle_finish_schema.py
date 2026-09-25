"""Validates data/fx/battle_finish.json against its schema (lot BV3: wind, standards, duels)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"


def _load(relative: str) -> dict:
    return json.loads((DATA / relative).read_text(encoding="utf-8"))


def test_battle_finish_matches_schema() -> None:
    """The battle finish file exists and matches its schema."""
    schema = _load("schemas/battle_finish.schema.json")
    settings = _load("fx/battle_finish.json")
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(settings), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_duel_clips_exist_in_the_rigs() -> None:
    """Every clip named by a duel sequence is baked in the matching rig."""
    settings = _load("fx/battle_finish.json")["duels"]
    manifest = json.loads(
        (ROOT / "game/assets/models/battle_skinned/manifest.json").read_text(
            encoding="utf-8"
        )
    )
    rigs = manifest["rigs"]
    for key, rig in (("sequence", "human"), ("mounted_sequence", "cavalry")):
        clips = set(rigs[rig]["clips"])
        for exchange in settings[key]:
            assert {exchange["a"], exchange["b"]} <= clips, (key, exchange)


def test_duel_types_exist() -> None:
    """Unit types allowed to duel are real unit types (or the general)."""
    settings = _load("fx/battle_finish.json")["duels"]
    unit_types = {path.stem for path in (DATA / "unit_types").glob("unit_*.json")}
    for entry in settings["enabled_for"]:
        assert entry == "general" or entry in unit_types, entry
