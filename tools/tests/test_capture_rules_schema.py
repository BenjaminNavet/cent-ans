"""Validates data/rules/capture.json against capture_rules.schema.json (lot TW2-T1, fate of captured places)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def _load(path: Path) -> dict | list:
    return json.loads(path.read_text(encoding="utf-8"))


def test_sack_pays_more_than_ransom() -> None:
    """Sacking takes more gold than a ransom, razing less than both."""
    rules = _load(DATA / "rules" / "capture.json")
    ransom = rules["ransom"]["gold_income_share"]
    sack = rules["sack"]["gold_income_share"]
    raze = rules["raze"]["effects"]["gold_income_share"]
    assert raze < ransom < sack


def test_forbidden_places_exist() -> None:
    """Every emblematic place that cannot be razed is a known settlement."""
    rules = _load(DATA / "rules" / "capture.json")
    known = set()
    for path in (DATA / "settlements").glob("prov_*.json"):
        known.update(item["id"] for item in _load(path))
    missing = [s for s in rules["raze_forbidden_settlements"] if s not in known]
    assert not missing, missing
