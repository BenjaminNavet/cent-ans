"""Tests of the banner and pennon generator (F10c)."""

import pytest
from PIL import Image

from cent_ans_tools import banners, heraldry


@pytest.fixture(scope="module")
def built(tmp_path_factory):
    """One build shared by the tests of this module (house banners make it slow)."""
    out_dir = tmp_path_factory.mktemp("banners")
    return out_dir, banners.build(out_dir=out_dir)


def test_build_writes_power_of_two_rgba_images(built):
    """Every faction and house gets a banner, a pennon and a standard, plus 3 specials."""
    _, paths = built
    factions = heraldry.load_factions()
    houses = heraldry.load_houses()["houses"]
    assert len(paths) == 3 * len(factions) + 3 * len(houses) + 3
    for path in paths:
        image = Image.open(path)
        assert image.mode == "RGBA"
        width, height = image.size
        assert width & (width - 1) == 0 and height & (height - 1) == 0
        if path.stem.endswith("_pennon"):
            expected = banners.PENNON_SIZE
        elif path.stem.endswith("_standard"):
            expected = banners.STANDARD_SIZE
        else:
            expected = banners.BANNER_SIZE
        assert image.size == expected


def test_banner_cloth_is_opaque_on_top_and_transparent_below(built):
    """The cloth hangs at the top of the texture, the rest is transparent."""
    tmp_path, _ = built
    image = Image.open(tmp_path / "fac_france_banner.png")
    assert image.getpixel((128, 100))[3] == 255
    assert image.getpixel((128, 500))[3] == 0


def test_france_banner_is_azure_with_gold(built):
    """The banner of France keeps its azure field."""
    tmp_path, _ = built
    colors = (
        Image.open(tmp_path / "fac_france_banner.png").convert("RGB").getcolors(1 << 16)
    )
    dominant = max(colors)[1]
    assert dominant[2] > dominant[0]  # azur field dominates


def test_standard_tapers_to_a_split_tail(built):
    """EP5: the long standard is opaque at the hoist, notched at the tail."""
    tmp_path, _ = built
    image = Image.open(tmp_path / "fac_england_standard.png")
    assert image.size == banners.STANDARD_SIZE
    assert image.getpixel((64, 128))[3] == 255
    assert image.getpixel((1020, 128))[3] == 0  # notch
    assert image.getpixel((1000, 4))[3] == 0  # tapered edge


def test_shield_polygon_is_restored_after_rendering():
    """Painting arms on a cloth must not leak the patched outline."""
    original = heraldry.shield_polygon
    banners.render_arms(heraldry.parse_blazon("croix", "#ffffff", "#b0182b"))
    assert heraldry.shield_polygon is original
