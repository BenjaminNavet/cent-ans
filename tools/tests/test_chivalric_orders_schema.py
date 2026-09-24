"""Validates data/chivalric_orders/*.json against chivalric_order.schema.json (H6)."""

import json
from pathlib import Path

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def test_every_chivalric_order_matches_the_schema() -> None:
    """Garter, Star, Golden Fleece and a generic order, each valid, file name equal to id."""
    validator = schema_validator(DATA, "chivalric_order.schema.json")
    files = sorted((DATA / "chivalric_orders").glob("*.json"))
    ids = set()
    for path in files:
        order = json.loads(path.read_text(encoding="utf-8"))
        errors = [error.message for error in validator.iter_errors(order)]
        assert not errors, f"{path.name}: {errors}"
        assert path.stem == order["id"]
        ids.add(order["id"])
    assert {"ord_garter", "ord_star", "ord_golden_fleece"} <= ids
    generic = [
        path
        for path in files
        if "faction" not in json.loads(path.read_text(encoding="utf-8"))
    ]
    assert generic, "a generic order is needed for the other factions"
