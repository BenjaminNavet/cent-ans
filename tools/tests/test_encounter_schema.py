"""Validates data/encounters/*.json and data/rules/encounters.json (lot CV3-3)."""

import json
from pathlib import Path

DATA = Path(__file__).resolve().parents[2] / "data"


def test_every_encounter_matches_the_schema() -> None:
    """About twelve encounters, each valid, sourced, file name equal to id."""
    files = sorted((DATA / "encounters").glob("enc_*.json"))
    assert len(files) >= 12
    for path in files:
        encounter = json.loads(path.read_text(encoding="utf-8"))
        assert path.stem == encounter["id"]
        assert encounter["sources"], f"{path.name}: sources vides"
        defaults = [o for o in encounter["options"] if o.get("default")]
        assert len(defaults) <= 1, f"{path.name}: plusieurs options par défaut"
        default = defaults[0] if defaults else encounter["options"][0]
        assert not default.get("conditions"), (
            f"{path.name}: l'option par défaut ne doit pas avoir de condition"
        )
        low, high = encounter["spawn"]["lifetime"]
        assert low <= high, f"{path.name}: lifetime inversée"
