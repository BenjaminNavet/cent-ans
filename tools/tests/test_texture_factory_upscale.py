# ruff: noqa: D103
"""Tests du module upscale de la fabrique de textures (TX T1d)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from PIL import Image

from cent_ans_tools.texture_factory import upscale as up


def _source(tmp_path: Path, side: int = 32) -> Path:
    path = tmp_path / "src.png"
    Image.effect_noise((side, side), 40).convert("RGB").save(path)
    return path


def test_lanczos_resizes_to_output_size(tmp_path: Path) -> None:
    dst = up.upscale(_source(tmp_path), tmp_path / "out" / "dst.png", "lanczos", 64)
    assert Image.open(dst).size == (64, 64)


def test_esrgan_uses_binary_and_resizes(tmp_path: Path, monkeypatch) -> None:
    binary = tmp_path / "realesrgan-ncnn-vulkan"
    binary.write_text("#!/bin/sh\n")
    binary.chmod(0o755)
    monkeypatch.setenv(up.BINARY_VARIABLE, str(binary))
    calls: list[list[str]] = []

    def fake_runner(command: list[str]) -> None:
        calls.append(command)
        Image.new("RGB", (128, 128), (10, 20, 30)).save(
            command[command.index("-o") + 1]
        )

    dst = up.upscale(_source(tmp_path), tmp_path / "dst.png", "esrgan", 64, fake_runner)
    command = calls[0]
    assert command[0] == str(binary)
    assert command[command.index("-n") + 1] == "realesrgan-x4plus"
    assert command[command.index("-s") + 1] == "4"
    assert Image.open(dst).size == (64, 64)


def test_esrgan_light_uses_x2_model() -> None:
    command = up.esrgan_command(Path("/b/bin"), "esrgan_light", Path("i"), Path("o"))
    assert command[command.index("-n") + 1] == "realesr-animevideov3"
    assert command[command.index("-s") + 1] == "2"
    assert command[command.index("-m") + 1] == "/b/models"


def test_missing_binary_is_a_clear_error(tmp_path: Path, monkeypatch) -> None:
    monkeypatch.setenv(up.BINARY_VARIABLE, str(tmp_path / "absent"))
    with pytest.raises(up.UpscaleError, match="CENT_ANS_REALESRGAN"):
        up.upscale(_source(tmp_path), tmp_path / "dst.png", "esrgan", 64)


def test_unknown_method(tmp_path: Path) -> None:
    with pytest.raises(up.UpscaleError, match="inconnue"):
        up.upscale(_source(tmp_path), tmp_path / "dst.png", "bicubic", 64)


def test_upscale_family_uses_catalogue_method(tmp_path: Path) -> None:
    entry = {"role": "r", "prompt": "p", "tile_m": 4, "biomes": [1]}
    document = {
        "family": "ground_campaign",
        "size": 1024,
        "output_size": 64,
        "upscale": "lanczos",
        "entries": [
            {**entry, "id": "a", "seed": 1},
            {**entry, "id": "b", "seed": 2},
        ],
    }
    Image.new("RGB", (32, 32), (1, 2, 3)).save(tmp_path / "a_2.png")
    manifest = {"a": {"attempt": 2, "seed": 2, "status": "ok"}}
    (tmp_path / "manifest.json").write_text(json.dumps(manifest))
    written = up.upscale_family(document, raw_dir=tmp_path)
    assert written == [tmp_path / "upscaled" / "a.png"]
    assert Image.open(written[0]).size == (64, 64)
    assert up.upscale_family({**document, "upscale": "native"}, raw_dir=tmp_path) == []
