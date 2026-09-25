"""Tests of the banner and pennon generator (F10c)."""

from PIL import Image

from cent_ans_tools import banners, heraldry


def test_build_writes_power_of_two_rgba_images(tmp_path):
    """Every faction gets a banner, a pennon and a standard, plus the three specials."""
    paths = banners.build(out_dir=tmp_path)
    factions = heraldry.load_factions()
    assert len(paths) == 3 * len(factions) + 3
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


def test_banner_cloth_is_opaque_on_top_and_transparent_below(tmp_path):
    """The cloth hangs at the top of the texture, the rest is transparent."""
    banners.build(out_dir=tmp_path)
    image = Image.open(tmp_path / "fac_france_banner.png")
    assert image.getpixel((128, 100))[3] == 255
    assert image.getpixel((128, 500))[3] == 0


def test_france_banner_is_azure_with_gold(tmp_path):
    """The banner of France keeps its azure field."""
    banners.build(out_dir=tmp_path)
    colors = (
        Image.open(tmp_path / "fac_france_banner.png").convert("RGB").getcolors(1 << 16)
    )
    dominant = max(colors)[1]
    assert dominant[2] > dominant[0]  # azur field dominates


def test_standard_tapers_to_a_split_tail(tmp_path):
    """EP5: the long standard is opaque at the hoist, notched at the tail."""
    banners.build(out_dir=tmp_path)
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
