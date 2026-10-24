"""Validates data/balance/seed_bots_bands.json against its schema."""

import json
from pathlib import Path

import pytest

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


@pytest.mark.parametrize("name", ["seed_bots_bands.json", "seed_bots_bands_quick.json"])
def test_seed_bots_bands_match_schema(name: str) -> None:
    """The bands files (long and quick pass) are valid and every band has min <= max when both are set."""
    document = json.loads((DATA / "balance" / name).read_text(encoding="utf-8"))
    validator = schema_validator(DATA, "seed_bots_bands.schema.json")
    validator.check_schema(validator.schema)
    errors = sorted(validator.iter_errors(document), key=lambda error: list(error.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors[:5]]
    for band in document.values():
        if band.get("min") is not None and band.get("max") is not None:
            assert band["min"] <= band["max"]
