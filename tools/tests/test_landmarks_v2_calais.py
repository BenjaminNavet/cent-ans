"""Calais around 1340 (lot VH8): dated key facts of data/landmarks_v2/calais.json."""

import json
from pathlib import Path

import pytest

DATA = Path(__file__).resolve().parents[2] / "data"


def _city() -> dict:
    return json.loads((DATA / "landmarks_v2" / "calais.json").read_text(encoding="utf-8"))


@pytest.mark.skip(reason="skeleton: facts not written yet")
def test_calais_1340_facts() -> None:
    """Placeholder."""
    assert _city()["id"] == "calais"
