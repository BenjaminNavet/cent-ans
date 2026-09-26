"""Tests of the battle capture saturation measurement (DA7b)."""

import numpy as np

from cent_ans_tools.scene_saturation import (
    VIEWS,
    mean_hue,
    measured_pixels,
    saturation_stats,
)


def test_grey_image_has_no_saturation() -> None:
    """A neutral grey capture measures 0 %."""
    image = np.full((900, 1600, 3), 128, dtype=np.uint8)
    stats = saturation_stats(image)
    assert stats.mean == 0.0
    assert abs(stats.value - 128 / 255) < 1e-9


def test_pure_colour_is_fully_saturated() -> None:
    """A pure red capture measures 100 %."""
    image = np.zeros((900, 1600, 3), dtype=np.uint8)
    image[..., 0] = 200
    assert saturation_stats(image).mean == 1.0


def test_measured_area_skips_the_sky_and_the_bottom_bars() -> None:
    """Only the 3D area below the horizon counts; the sky and the HUD bars are ignored."""
    image = np.full((900, 1600, 3), 100, dtype=np.uint8)
    image[:180] = (40, 90, 220)  # sky
    image[650:] = (220, 30, 30)  # bottom bars
    assert saturation_stats(image).mean == 0.0
    assert measured_pixels(image).shape == (390 * 1040 + 80 * 1240, 3)


def test_other_sizes_are_rescaled() -> None:
    """A half-size capture keeps the same measured proportion."""
    image = np.full((450, 800, 3), 60, dtype=np.uint8)
    assert measured_pixels(image).shape == (195 * 520 + 40 * 620, 3)


def test_views_cover_the_da6_table() -> None:
    """Every DA6 view is known, bocage and autumn included."""
    assert {"haute", "ligne", "hiver", "bocage", "automne", "closeup"} <= set(VIEWS)


def test_mean_hue_of_primaries() -> None:
    """Red, green and blue sit at 0°, 120° and 240°; an orange at about 30°."""
    assert abs(mean_hue(np.array([[1.0, 0.0, 0.0]]))) < 1e-6
    assert abs(mean_hue(np.array([[0.0, 1.0, 0.0]])) - 120.0) < 1e-6
    assert abs(mean_hue(np.array([[0.0, 0.0, 1.0]])) - 240.0) < 1e-6
    assert 25.0 < mean_hue(np.array([[1.0, 0.5, 0.0]])) < 35.0
    assert np.isnan(mean_hue(np.zeros((0, 3))))
