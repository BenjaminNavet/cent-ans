# ruff: noqa: D103
"""Tests de la passe raccord/PBR et des paquets d'une famille (TX T2b3)."""

from __future__ import annotations

import json

import numpy as np
from PIL import Image

from cent_ans_tools.texture_factory import process

DOCUMENT = {
    "family": "ground_campaign",
    "processing": {"blend_width": 256},
    "entries": [
        {"id": "bg_grass_b01", "role": "grass_bare", "biomes": [1], "tile_m": 100},
        {"id": "bg_rock_b01", "role": "rock", "biomes": [1], "tile_m": 80},
        {"id": "parcel_wheat", "role": "parcel_crop", "biomes": [2], "tile_m": 40},
        {"id": "parcel_rye", "role": "parcel_crop", "biomes": [2], "tile_m": 40},
    ],
    "packs": [
        {
            "name": "parcels",
            "roles": ["parcel_crop"],
            "texture_dir": "assets/tex/",
            "hi_dir": "hi/",
            "albedo": "p_albedo.jpg",
            "normal": "p_normal.jpg",
            "manifest": "data/p_pack.json",
            "layer_size": 64,
            "normal_size": 32,
            "strip_prefix": "parcel_",
        }
    ],
}


def _raw(tmp_path):
    raw = tmp_path / "raw"
    raw.mkdir()
    rng = np.random.default_rng(1)
    manifest = {}
    for entry in DOCUMENT["entries"]:
        pixels = rng.integers(40, 200, (128, 128, 3), dtype=np.uint8)
        Image.fromarray(pixels).save(raw / f"{entry['id']}_2.png")
        manifest[entry["id"]] = {"attempt": 2, "seed": 1, "status": "ok"}
    manifest["parcel_rye"]["status"] = "flagged"
    (raw / "manifest.json").write_text(json.dumps(manifest))
    return raw


def test_pack_entries_keep_ok_roles_and_attempt(tmp_path):
    raw = _raw(tmp_path)
    manifest = json.loads((raw / "manifest.json").read_text())
    entries = process.pack_entries(DOCUMENT, DOCUMENT["packs"][0], manifest)
    assert [(e["id"], e["attempt"]) for e in entries] == [("parcel_wheat", 2)]


def test_build_pack_writes_arrays_and_stripped_manifest(tmp_path):
    raw = _raw(tmp_path)
    result = process.build_packs(DOCUMENT, raw_dir=raw, repo_root=tmp_path)
    assert result["parcels"]["layers"] == 1
    assert (tmp_path / "game/assets/tex/p_albedo.jpg").is_file()
    data = json.loads((tmp_path / "data/p_pack.json").read_text())
    assert data["albedo"] == "res://assets/tex/p_albedo.jpg"
    assert [layer["id"] for layer in data["layers"]] == ["wheat"]
    assert data["normal_size"] == 32
    hi = process.build_packs(DOCUMENT, raw_dir=raw, repo_root=tmp_path, size=128)
    assert hi["parcels"]["layers"] == 1
    data = json.loads((tmp_path / "data/p_pack_128.json").read_text())
    assert data["albedo"] == "res://assets/tex/hi/p_albedo.jpg"
    assert data["layer_size"] == 128 and data["normal_size"] == 64
