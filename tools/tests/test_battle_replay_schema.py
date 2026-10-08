"""Validates the battle replay rules and a sample replay file against their schemas (EP13)."""

import json
from pathlib import Path

from jsonschema import Draft202012Validator

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
SAMPLE = (
    ROOT
    / "core"
    / "crates"
    / "sim-battle"
    / "tests"
    / "fixtures"
    / "replay_sample.json"
)


def _errors(schema_name: str, document: dict) -> list[str]:
    schema = json.loads((DATA / "schemas" / schema_name).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(document), key=lambda e: e.path
    )
    return [error.message for error in errors]


def test_replay_periods_cover_a_long_battle() -> None:
    """Enough copies for a 30 min battle at the chosen period (bounded memory)."""
    rules = json.loads(
        (DATA / "rules" / "battle_replay.json").read_text(encoding="utf-8")
    )
    assert rules["max_keyframes"] * rules["keyframe_seconds"] >= 1800
    assert rules["checkpoint_seconds"] <= rules["keyframe_seconds"]


def test_sample_replay_matches_schema() -> None:
    """The sample written by `sim-battle` (test `write_sample`) matches `battle_replay.schema.json`."""
    sample = json.loads(SAMPLE.read_text(encoding="utf-8"))
    assert not _errors("battle_replay.schema.json", sample)
    assert sample["actions"], "the sample records player inputs"
    assert sample["end"]["outcome"]["winner"] in ("attacker", "defender")
