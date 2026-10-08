"""Validates data/ai/doctrines.json against ai_doctrine.schema.json (lot E1)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_ai_doctrines_match_schema() -> None:
    """The recruitment doctrines file exists, matches its schema and names real ids."""
    doctrines = json.loads((DATA / "ai" / "doctrines.json").read_text(encoding="utf-8"))
    units = {path.stem for path in (DATA / "unit_types").glob("*.json")}
    factions = {path.stem for path in (DATA / "factions").glob("*.json")}
    all_doctrines = [doctrines["default"], *doctrines.get("factions", {}).values()]
    for doctrine in all_doctrines:
        assert set(doctrine["mix"]) <= units
    assert set(doctrines.get("factions", {})) <= factions
    assert set(doctrines.get("share_caps", {})) <= units
