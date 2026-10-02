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


def _digest_image(image: Image.Image):
    return hashlib.sha256(image.tobytes()).hexdigest()


def test_new_charges_render_distinct_from_plain_field() -> None:
    """FE new charges (key, cauldron, hand, ship, crescent, ox, chief) are drawn.

    Each faction blazon must give a shield distinct from a plain field of the same
    tinctures (regression for FEH: these blazons used to fall through to a bare
    field and collide with each other).
    """
    factions = {faction["id"]: faction for faction in heraldry.load_factions()}
    new_charge_factions = [
        "fac_bremen",
        "fac_lara",
        "fac_tyrone",
        "fac_isles",
        "fac_luna",
        "fac_ormond",
        "fac_urgell",
    ]
    for faction_id in new_charge_factions:
        heraldry_data = factions[faction_id]["heraldry"]
        blazon = heraldry.parse_blazon(
            heraldry_data["blazon"],
            heraldry_data["primary_color"],
            heraldry_data["secondary_color"],
        )
        charged = heraldry.render_shield(blazon)
        plain = heraldry.Blazon(field=blazon.field, charge=blazon.charge, text="")
        bare = heraldry.render_shield(plain)
        assert _digest_image(charged) != _digest_image(bare), faction_id


def test_jerusalem_cross_has_gold_in_four_cantons() -> None:
    """The crusader shield shows a potent cross and a crosslet in each canton."""
    faction = {
        "heraldry": {
            "blazon": "D'argent à la croix potencée d'or cantonnée de quatre croisettes du même.",
            "primary_color": "#F5F1E6",
            "secondary_color": "#C9A227",
        }
    }
    image = heraldry.shield_for_faction(faction).convert("RGB")
    size = image.width

    def is_gold(fx: float, fy: float) -> bool:
        red, green, blue = image.getpixel((int(fx * size), int(fy * size)))
        return red > 150 and blue < 90 and green > 110

    for canton in ((0.31, 0.29), (0.69, 0.29), (0.31, 0.61), (0.69, 0.61)):
        assert is_gold(*canton)
    assert is_gold(0.5, 0.45)
    assert not is_gold(0.2, 0.2)
