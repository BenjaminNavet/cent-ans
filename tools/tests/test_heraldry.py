"""Tests for the procedural heraldry generator."""

import hashlib
from pathlib import Path

from PIL import Image

from cent_ans_tools import heraldry


def _digest(path: Path) -> str:
    return hashlib.sha256(Image.open(path).tobytes()).hexdigest()


def test_parse_blazon_field_and_charges() -> None:
    """The field tincture comes from the blazon, charges from the secondary color."""
    blazon = heraldry.parse_blazon(
        "D'azur semé de fleurs de lis d'or.", "#000000", "#F2C230"
    )
    assert blazon.field == heraldry.TINCTURES["azur"]
    assert blazon.charge == (0xF2, 0xC2, 0x30)
    assert blazon.has("semé")
    assert (
        heraldry.parse_blazon(
            "D'or à quatre pals de gueules.", "#F2C230", "#B0182B"
        ).count(1)
        == 4
    )


def test_parse_blazon_falls_back_to_primary() -> None:
    """An unknown opening word keeps the primary color as field."""
    blazon = heraldry.parse_blazon("Bandé d'or et d'azur.", "#123456", "#FFFFFF")
    assert blazon.field == (0x12, 0x34, 0x56)


def test_build_is_deterministic(tmp_path: Path) -> None:
    """Two builds produce identical pixels, one 128x128 RGBA shield per faction."""
    first = heraldry.build(out_dir=tmp_path / "a")
    second = heraldry.build(out_dir=tmp_path / "b")
    assert len(first) == len(list(heraldry.FACTIONS_DIR.glob("*.json")))
    for path_a, path_b in zip(first, second, strict=True):
        image = Image.open(path_a)
        assert image.size == (128, 128)
        assert image.mode == "RGBA"
        assert _digest(path_a) == _digest(path_b)


def test_shields_are_distinct_and_masked(tmp_path: Path) -> None:
    """Corners are transparent (shield mask) and shields differ between factions."""
    paths = heraldry.build(out_dir=tmp_path)
    digests = {_digest(path) for path in paths}
    assert len(digests) == len(paths)
    image = Image.open(paths[0])
    assert image.getpixel((127, 127))[3] == 0
    assert image.getpixel((64, 40))[3] == 255
