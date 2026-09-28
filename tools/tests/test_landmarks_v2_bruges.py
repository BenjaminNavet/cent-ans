"""Bruges around 1340 (lot VH8): key dated facts of data/landmarks_v2/bruges.json."""

import json
from pathlib import Path

import pytest

CITY = Path(__file__).resolve().parents[2] / "data" / "landmarks_v2" / "bruges.json"


@pytest.mark.skip(reason="VH8 skeleton: facts not written yet")
def test_bruges_1340_facts() -> None:
    """Placeholder until the city is written."""
    city = json.loads(CITY.read_text(encoding="utf-8"))
    assert city["id"] == "bruges"
