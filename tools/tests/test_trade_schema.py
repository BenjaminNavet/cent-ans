"""Validates data/economy/trade.json against trade.schema.json (lot C5)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _trade() -> dict:
    return json.loads((DATA / "economy" / "trade.json").read_text(encoding="utf-8"))


def test_routes_reference_known_hubs() -> None:
    """Every route's `from_hub`/`to_hub` names a hub of the catalogue."""
    trade = _trade()
    hub_ids = {hub["id"] for hub in trade["hubs"]}
    for route in trade["routes"]:
        assert route["from_hub"] in hub_ids, route["id"]
        assert route["to_hub"] in hub_ids, route["id"]
