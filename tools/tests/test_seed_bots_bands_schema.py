"""Validates data/balance/seed_bots_bands.yaml against its schema."""

from pathlib import Path

import yaml

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_seed_bots_bands_match_schema() -> None:
    """The bands file is valid and every band has min <= max when both are set."""
    document = yaml.safe_load((DATA / "balance" / "seed_bots_bands.yaml").read_text(encoding="utf-8"))
    validator = schema_validator(DATA, "seed_bots_bands.schema.json")
    validator.check_schema(validator.schema)
    errors = sorted(validator.iter_errors(document), key=lambda error: list(error.path))
    assert not errors, [f"{list(e.path)}: {e.message}" for e in errors[:5]]
    for band in document.values():
        if band.get("min") is not None and band.get("max") is not None:
            assert band["min"] <= band["max"]
