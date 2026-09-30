"""NB: tests of the UI ornament pipeline (no network)."""

import json
from decimal import Decimal
from pathlib import Path

import numpy as np
from jsonschema import Draft202012Validator
from PIL import Image

from cent_ans_tools import openrouter, ui_illumination, ui_ornaments

ROOT = Path(__file__).resolve().parents[2]
PIECE = {"size": [60, 40], "margins": [10, 8, 10, 8], "content": [4, 4, 4, 4]}


def _green_with_gold() -> np.ndarray:
    image = np.zeros((50, 50, 3), np.uint8)
    image[:] = (0, 255, 0)
    image[10:40, 10:40] = (230, 190, 60)  # gold
    image[10:40, 10:12] = (120, 200, 90)  # greenish fringe
    return image


def test_key_out_removes_green() -> None:
    """No green-dominant pixel is left opaque after keying; gold stays opaque."""
    out = ui_ornaments.key_out(_green_with_gold())
    assert out.shape == (50, 50, 4)
    assert out[0, 0, 3] == 0
    assert (out[20:40, 20:40, 3] == 255).all()
    assert tuple(out[25, 25, :3]) == (230, 190, 60)
    visible = out[..., 3] > 0
    dominant = out[..., 1].astype(int) - np.maximum(out[..., 0], out[..., 2])
    assert (dominant[visible] <= 0).all()


def test_fit_to_piece_matches_kit_size() -> None:
    """Output size equals the kit.json piece size."""
    out = ui_ornaments.fit_to_piece(ui_ornaments.key_out(_green_with_gold()), PIECE)
    assert out.shape == (40, 60, 4)


def test_seam_fix_seam_error() -> None:
    """Seam error of the stretched bands is at most 4/255; corners are untouched."""
    rng = np.random.default_rng(3)
    image = rng.integers(0, 256, (40, 60, 4), dtype=np.uint8)
    out = ui_ornaments.seam_fix(image, PIECE)
    left, top, right, bottom = PIECE["margins"]
    band_x = out[:, left : 60 - right].astype(int)
    assert np.abs(band_x[:, 0] - band_x[:, -1]).max() <= 4
    band_y = out[top : 40 - bottom].astype(int)
    assert np.abs(band_y[0] - band_y[-1]).max() <= 4
    assert (out[:top, :left] == image[:top, :left]).all()
    assert (out[-bottom:, -right:] == image[-bottom:, -right:]).all()


def test_fallback_is_procedural(tmp_path: Path) -> None:
    """A missing NB piece leaves the procedural drawing unchanged (and is committed)."""
    written = ui_illumination.build(tmp_path)
    committed = ui_illumination.OUTPUT_DIR
    for path in written:
        assert path.read_bytes() == (committed / path.name).read_bytes(), path.name
    assert (tmp_path / "kit.json").read_text() == (committed / "kit.json").read_text()


def test_nb_override_and_size_check(tmp_path: Path) -> None:
    """An nb/<id>.png replaces the piece; a wrong size raises."""
    kit = json.loads((ui_illumination.OUTPUT_DIR / "kit.json").read_text())
    name = next(iter(kit))
    width, height = kit[name]["size"]
    (tmp_path / "nb").mkdir()
    Image.new("RGBA", (width, height), (1, 2, 3, 255)).save(
        tmp_path / "nb" / f"{name}.png"
    )
    ui_illumination.build(tmp_path)
    assert Image.open(tmp_path / f"{name}.png").getpixel((0, 0)) == (1, 2, 3, 255)
    Image.new("RGBA", (width + 1, height), (1, 2, 3, 255)).save(
        tmp_path / "nb" / f"{name}.png"
    )
    try:
        ui_illumination.build(tmp_path)
    except ValueError:
        pass
    else:
        raise AssertionError("expected ValueError")


def _config() -> dict:
    kit = json.loads((ui_illumination.OUTPUT_DIR / "kit.json").read_text())
    name = next(iter(kit))
    return {
        "model": "m/x",
        "guide_scale": 2,
        "pieces": [
            {
                "id": name,
                "prompt": "P",
                "aspect_ratio": "1:1",
                "image_size": "1K",
                "variants": 3,
                "seed": 10,
            },
            {
                "id": "not_in_kit",
                "prompt": "Q",
                "aspect_ratio": "1:1",
                "image_size": "1K",
                "variants": 2,
            },
        ],
        "decor": [
            {
                "id": "flourish",
                "prompt": "D",
                "aspect_ratio": "3:2",
                "image_size": "1K",
                "variants": 2,
            },
        ],
    }


def test_build_requests() -> None:
    """Counts, references, seeds and suffixes."""
    kit = json.loads((ui_illumination.OUTPUT_DIR / "kit.json").read_text())
    requests = ui_ornaments.build_requests(kit, b"ANCHOR", _config())
    pieces = [r for r in requests if r.piece_id != "flourish"]
    decor = [r for r in requests if r.piece_id == "flourish"]
    assert len(pieces) == 3 and len(decor) == 2
    assert [r.seed for r in pieces] == [10, 11, 12]
    assert [r.seed for r in decor] == [1337, 1338]
    assert all(len(r.references) == 2 and r.references[0] == b"ANCHOR" for r in pieces)
    assert all(r.references == (b"ANCHOR",) for r in decor)
    assert pieces[0].prompt.endswith(ui_ornaments.GUIDE_SUFFIX)
    assert decor[0].prompt.endswith(ui_ornaments.DECOR_SUFFIX)
    assert pieces[0].image_config == {"aspect_ratio": "1:1", "image_size": "1K"}


def test_generate_dry_run_makes_no_call(monkeypatch, tmp_path: Path, capsys) -> None:
    """Dry run estimates but never generates."""

    def boom(*args, **kwargs):
        raise AssertionError("network call")

    monkeypatch.setattr(openrouter, "generate_image", boom)
    monkeypatch.setattr(
        openrouter,
        "estimate_price",
        lambda model, client=None, image_size=None: Decimal("0.07"),
    )
    kit = json.loads((ui_illumination.OUTPUT_DIR / "kit.json").read_text())
    requests = ui_ornaments.build_requests(kit, b"A", _config())
    paths = ui_ornaments.generate(requests, tmp_path / "raw", dry_run=True, model="m/x")
    assert len(paths) == len(requests)
    assert "0.35" in capsys.readouterr().out


def test_generate_skips_existing(monkeypatch, tmp_path: Path) -> None:
    """Existing raw files are reused; the others are generated with the seed."""
    calls = []
    monkeypatch.setattr(
        openrouter, "generate_image", lambda model, prompt, path, **kw: calls.append(kw)
    )
    kit = json.loads((ui_illumination.OUTPUT_DIR / "kit.json").read_text())
    requests = ui_ornaments.build_requests(kit, b"A", _config())
    raw = tmp_path / "raw"
    raw.mkdir()
    first = requests[0]
    (raw / f"{first.piece_id}_v{first.variant}.png").write_bytes(b"x")
    ui_ornaments.generate(requests, raw, model="m/x")
    assert len(calls) == len(requests) - 1
    assert calls[0]["cap"] == ui_ornaments.NB_CAP


def test_contact_sheet_small(tmp_path: Path) -> None:
    """The sheet is written and stays under 1 MB."""
    image = Image.new("RGBA", (40, 30), (200, 150, 50, 255))
    image.save(tmp_path / "a.png")
    out = ui_ornaments.contact_sheet(
        [tmp_path / "a.png"], [tmp_path / "a.png"], tmp_path / "s.png"
    )
    assert out.stat().st_size < 1_000_000


def test_ui_ornaments_match_schema() -> None:
    """``data/art/ui_ornaments.yaml`` matches its schema."""
    schema = json.loads(
        (ROOT / "data" / "schemas" / "ui_ornaments.schema.json").read_text(
            encoding="utf-8"
        )
    )
    Draft202012Validator.check_schema(schema)
    errors = list(
        Draft202012Validator(schema).iter_errors(ui_ornaments.load_ornaments())
    )
    assert not errors, [e.message for e in errors]
