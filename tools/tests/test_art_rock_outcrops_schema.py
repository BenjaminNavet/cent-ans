"""Validates the HB5 rock outcrop catalogue against its schema (ADR 0143)."""

import json
from pathlib import Path

import yaml

from cent_ans_tools.codex import schema_validator

DATA = Path(__file__).resolve().parents[2] / "data"
MODELS = Path(__file__).resolve().parents[2] / "game/assets/models/rocks/hb"


def _document() -> dict:
    return yaml.safe_load(
        (DATA / "art" / "rock_outcrops.yaml").read_text(encoding="utf-8")
    )


def test_rock_outcrops_match_schema() -> None:
    """``data/art/rock_outcrops.yaml`` matches ``art_rock_outcrops.schema.json``."""
    validator = schema_validator(DATA, "art_rock_outcrops.schema.json")
    errors = sorted(validator.iter_errors(_document()), key=lambda e: e.path)
    assert not errors, [error.message for error in errors]


def test_rock_outcrops_godot_readable() -> None:
    """Godot reads the file as JSON once the ``#`` comment lines are dropped."""
    text = (DATA / "art" / "rock_outcrops.yaml").read_text(encoding="utf-8")
    body = "\n".join(
        line for line in text.splitlines() if not line.lstrip().startswith("#")
    )
    assert json.loads(body) == _document()


def test_rock_outcrops_consistent() -> None:
    """Unique ids, ordered bounds, every biome 1-7 covered."""
    document = _document()
    outcrops = document["outcrops"]
    ids = [o["id"] for o in outcrops]
    assert len(ids) == len(set(ids))
    for entry in outcrops:
        assert entry["size_m"][0] <= entry["size_m"][1], entry["id"]
        assert entry["min_altitude_m"] < entry["max_altitude_m"], entry["id"]
    covered = {b for o in outcrops for b in o["biomes"]}
    assert covered == set(range(1, 8))
    render = document["render"]
    assert render["lod_distances"][0] < render["lod_distances"][1]
    assert render["fade_start"] < render["fade_end"] <= 1200.0


def test_rock_outcrop_models_present() -> None:
    """Each catalogued outcrop has its three LOD files once the models are produced."""
    if not MODELS.exists():
        return
    for entry in _document()["outcrops"]:
        for lod in range(3):
            assert (MODELS / f"{entry['id']}_lod{lod}.glb").exists(), entry["id"]
