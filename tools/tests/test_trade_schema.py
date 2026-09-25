"""Validates data/economy/trade.json against trade.schema.json (lot C5)."""

import json
from pathlib import Path

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"


def _trade() -> dict:
    return json.loads((DATA / "economy" / "trade.json").read_text(encoding="utf-8"))


def test_trade_matches_the_schema() -> None:
    """The trade catalogue exists and is valid."""
    validator = schema_validator(DATA, "trade.schema.json")
    errors = [error.message for error in validator.iter_errors(_trade())]
    assert not errors, errors


def test_routes_reference_known_hubs() -> None:
    """Every route's `from_hub`/`to_hub` names a hub of the catalogue."""
    trade = _trade()
    hub_ids = {hub["id"] for hub in trade["hubs"]}
    for route in trade["routes"]:
        assert route["from_hub"] in hub_ids, route["id"]
        assert route["to_hub"] in hub_ids, route["id"]
