"""Validates data/retinue.json against retinue.schema.json (lot C7)."""

import json
from pathlib import Path

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_retinue_matches_the_schema() -> None:
    """The retinue catalogue exists, is valid and has unique ids."""
    validator = schema_validator(DATA, "retinue.schema.json")
    retinue = json.loads((DATA / "retinue.json").read_text(encoding="utf-8"))
    errors = [error.message for error in validator.iter_errors(retinue)]
    assert not errors, errors
    ids = [companion["id"] for companion in retinue["companions"]]
    assert len(ids) == len(set(ids))
    assert len(ids) >= 12


def test_building_triggers_name_a_building() -> None:
    """Every season trigger names an existing building, and only those do."""
    retinue = json.loads((DATA / "retinue.json").read_text(encoding="utf-8"))
    for companion in retinue["companions"]:
        for rule in companion["acquisition"]:
            seasonal = rule["trigger"] == "season_in_settlement"
            assert seasonal == ("building" in rule), companion["id"]
            if seasonal:
                assert (DATA / "buildings" / f"{rule['building']}.json").is_file()
