# ruff: noqa: D103
"""Tests alpha et micro-détail de la fabrique de textures (TX T1e)."""

from __future__ import annotations

import json
from pathlib import Path

import numpy as np
from PIL import Image

from cent_ans_tools.texture_factory import alpha, checks, pbr


def _fake_remover(image: Image.Image) -> Image.Image:
    rgba = image.convert("RGBA")
    mask = np.zeros((image.height, image.width), np.uint8)
    mask[8:24, 8:24] = 255
    rgba.putalpha(Image.fromarray(mask))
    return rgba


def _noise(path: Path, side: int = 64, seed: int = 0) -> Path:
    rng = np.random.default_rng(seed)
    Image.fromarray(rng.integers(0, 255, (side, side, 3), dtype=np.uint8)).save(path)
    return path


def test_cut_out_writes_rgba(tmp_path: Path) -> None:
    src = _noise(tmp_path / "s.png", 32)
    dst = alpha.cut_out(src, tmp_path / "o" / "d.png", remover=_fake_remover)
    image = Image.open(dst)
    assert image.mode == "RGBA" and image.size == (32, 32)


def test_alpha_family_only_alpha_entries(tmp_path: Path, monkeypatch) -> None:
    document = {"family": "x"}
    _noise(tmp_path / "leaf_1.png", 32)
    _noise(tmp_path / "soil_1.png", 32)
    manifest = {
        "leaf": {"attempt": 1, "status": "ok", "seed": 1},
        "soil": {"attempt": 1, "status": "ok", "seed": 2},
    }
    (tmp_path / "manifest.json").write_text(json.dumps(manifest))
    monkeypatch.setattr(
        "cent_ans_tools.texture_factory.catalog.select",
        lambda doc, only=None: [{"id": "leaf", "alpha": True}, {"id": "soil"}],
    )
    written = alpha.alpha_family(document, raw_dir=tmp_path, remover=_fake_remover)
    assert written == [tmp_path / "alpha" / "leaf.png"]
    assert written[0].is_file()
    assert not (tmp_path / "alpha" / "soil.png").exists()


def test_fill_ratio_and_border() -> None:
    rgba = np.zeros((32, 32, 4), np.uint8)
    rgba[8:24, 8:24, 3] = 255
    assert checks.alpha_fill_ratio(rgba) == 0.25
    assert not checks.alpha_touches_border(rgba)
    rgba[0, 5, 3] = 255
    assert checks.alpha_touches_border(rgba)


def test_micro_detail(tmp_path: Path) -> None:
    sources = [_noise(tmp_path / f"n{i}.png", 128, i) for i in range(3)]
    dst = pbr.micro_detail(sources, tmp_path / "m.png", size=128)
    data = np.asarray(Image.open(dst))
    assert data.shape == (128, 128) and data.dtype == np.uint8
    unit = data / 255.0
    assert abs(unit.mean() - 0.5) < 0.03
    assert 0.06 < unit.std() < 0.2
    assert checks.seam_ratio(unit) < 1.5


def test_micro_detail_normal(tmp_path: Path) -> None:
    source = _noise(tmp_path / "n.png", 128)
    dst = pbr.micro_detail_normal([source], tmp_path / "n_n.png", size=128)
    image = Image.open(dst)
    assert image.mode == "RGB" and image.size == (128, 128)
