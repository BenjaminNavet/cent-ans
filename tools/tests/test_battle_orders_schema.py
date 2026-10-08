"""Validates data/battle_orders/*.json against battle_order.schema.json (F10b)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_every_battle_order_matches_the_schema() -> None:
    """The four leader's orders (the pavise became an ability, CB4), each valid."""
    files = sorted((DATA / "battle_orders").glob("*.json"))
    kinds = set()
    for path in files:
        order = json.loads(path.read_text(encoding="utf-8"))
        assert path.stem == order["id"]
        kinds.add(order["kind"])
    assert kinds == {"war_cry", "no_quarter", "dismount", "rally"}


def test_faction_labels_name_existing_factions() -> None:
    """Every faction named in labels or AI rivals exists in data/factions."""
    factions = {path.stem for path in (DATA / "factions").glob("*.json")}
    for path in (DATA / "battle_orders").glob("*.json"):
        order = json.loads(path.read_text(encoding="utf-8"))
        named = set(order.get("labels_by_faction", {}))
        for pair in order.get("ai", {}).get("rivals", []):
            named.update(pair)
        assert named <= factions, f"{path.name}: {named - factions}"
