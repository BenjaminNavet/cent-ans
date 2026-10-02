"""Validates the FA6 parchment ornament catalogue against its schema."""

import json
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import parchment_ornaments

ROOT = Path(__file__).resolve().parents[2]
DATA = ROOT / "data"
# Share of the half side taken by the circle of the rose (`PM_ROSE_CIRCLE`, parchment_sea.gdshaderinc).
ROSE_CIRCLE = 0.9
# Budget of the whole output folder (lot brief).
MAX_BYTES = 4 * 1024 * 1024


def _document() -> dict:
    return json.loads(
        (DATA / "map" / "parchment_ornaments.json").read_text(encoding="utf-8")
    )


def test_parchment_ornaments_match_schema() -> None:
    """``data/map/parchment_ornaments.json`` matches its schema."""
    schema = json.loads(
        (DATA / "schemas" / "map_parchment_ornaments.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = sorted(
        Draft202012Validator(schema).iter_errors(_document()), key=lambda e: e.path
    )
    assert not errors, [error.message for error in errors]


def test_parchment_ornaments_are_consistent() -> None:
    """Sources exist, ids and files are unique, masks lie inside their crop."""
    document = _document()
    ornaments = document["ornaments"]
    assert len({o["id"] for o in ornaments}) == len(ornaments)
    assert len({o["file"] for o in ornaments}) == len(ornaments)
    for ornament in ornaments:
        assert ornament["source"] in document["sources"], ornament["id"]
        x0, y0, x1, y1 = ornament["crop"]
        assert x1 > x0 and y1 > y0, ornament["id"]
        mask = ornament["mask"]
        if mask["shape"] == "polygon":
            for x, y in mask["points"]:
                assert x0 <= x <= x1 and y0 <= y <= y1, ornament["id"]
        if ornament["kind"] != "rose":
            assert "height" in ornament["display"], ornament["id"]


def test_every_kind_has_a_real_ornament() -> None:
    """Switching a kind to ``real`` in ``use`` always finds a cut-out to show."""
    document = _document()
    kinds = {ornament["kind"] for ornament in document["ornaments"]}
    assert set(document["use"]) <= kinds


def test_rose_follows_the_shader_convention() -> None:
    """A rose is a centred square crop whose circle takes 0.9 of the half side."""
    for ornament in _document()["ornaments"]:
        if ornament["kind"] != "rose":
            continue
        x0, y0, x1, y1 = ornament["crop"]
        half = (x1 - x0) / 2
        assert x1 - x0 == y1 - y0
        assert ornament["mask"]["shape"] == "disk"
        assert ornament["mask"]["center"] == [x0 + half, y0 + half]
        circle = ornament["key"]["reject_except"]["radius"]
        assert abs(circle / half - ROSE_CIRCLE) < 0.02


def test_parchment_ornament_textures_exist() -> None:
    """Every ornament has its built texture, credited in ``SOURCE.md``, within budget."""
    document = _document()
    folder = ROOT / document["output_dir"]
    notes = (folder / "SOURCE.md").read_text(encoding="utf-8")
    assert notes == parchment_ornaments.source_notes(document)
    for ornament in document["ornaments"]:
        path = folder / ornament["file"]
        assert path.is_file(), ornament["id"]
        with Image.open(path) as image:
            assert image.mode == "RGBA"
            assert max(image.size) == ornament["size"]
    assert sum(p.stat().st_size for p in folder.glob("*.png")) <= MAX_BYTES


def test_ink_key_removes_paper_and_keeps_ink() -> None:
    """The vellum becomes transparent, a dark stroke stays opaque, blue waves are rejected."""
    rgb = np.full((64, 64, 3), 0.85, dtype=np.float32)
    rgb[30:34, 8:56] = (0.2, 0.12, 0.08)  # ink stroke
    rgb[10:12, 8:56] = (0.55, 0.62, 0.85)  # blue wave
    key = {"low": 0.1, "high": 0.3, "paper_radius": 16, "reject_blue": [0.02, 0.08]}
    alpha, colour = parchment_ornaments.ink_key(rgb, key, (0, 0))
    assert alpha[50, 32] < 0.02
    assert alpha[32, 32] > 0.98
    assert alpha[11, 32] < 0.05
    assert colour[32, 32].max() < 0.3


def test_shape_mask_polygon_with_hole() -> None:
    """A polygon mask is opaque inside, transparent outside and in its holes."""
    shape = {
        "shape": "polygon",
        "points": [[10, 10], [50, 10], [50, 50], [10, 50]],
        "holes": [[[25, 25], [35, 25], [35, 35], [25, 35]]],
    }
    mask = parchment_ornaments.shape_mask(shape, (0, 0), (64, 64))
    assert mask[15, 15] == 1.0
    assert mask[30, 30] == 0.0
    assert mask[60, 60] == 0.0
