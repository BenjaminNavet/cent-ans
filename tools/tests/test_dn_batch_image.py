"""Entrées `image` de dn_batch : prompt verbatim, taille fal, recadrage vers ingest.size."""

import importlib.util
from pathlib import Path

from PIL import Image

SPEC = importlib.util.spec_from_file_location(
    "dn_batch", Path(__file__).resolve().parents[1] / "experiments" / "dn_batch.py"
)
dn_batch = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(dn_batch)

ENTRY = {
    "id": "illus_test",
    "kind": "image",
    "prompt": "A miniature.",
    "ingest": {"out": "res://assets/illustrations/_test_dn.jpg", "size": [1024, 576]},
}


def test_prompt_is_verbatim():
    assert dn_batch.full_prompt(ENTRY) == "A miniature."


def test_fal_size_is_multiple_of_16():
    assert dn_batch.image_size(ENTRY) == {"width": 1024, "height": 576}
    odd = {**ENTRY, "ingest": {**ENTRY["ingest"], "size": [1000, 570]}}
    assert dn_batch.image_size(odd) == {"width": 1008, "height": 576}


def test_finalise_crops_and_writes_jpg(tmp_path, monkeypatch):
    monkeypatch.setattr(dn_batch, "REPO", tmp_path)
    source = tmp_path / "src.png"
    Image.new("RGB", (1024, 1024), (200, 30, 30)).save(source)
    target = dn_batch.finalise_image(ENTRY, source)
    assert target == tmp_path / "game/assets/illustrations/_test_dn.jpg"
    assert Image.open(target).size == (1024, 576)
